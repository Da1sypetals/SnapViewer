#!/usr/bin/env python3
"""
SnapViewer GUI using TKinter
Features:
- Left panel: Displays messages from renderer process
- Right panel: SQLite REPL
- Renderer process runs OpenGL window
- UI process runs Tkinter GUI
- Communication via ZeroMQ IPC
"""

import ctypes
import os
import platform
import subprocess
import sys
import threading
import time
import tkinter as tk
from ctypes import wintypes
from datetime import datetime
from pathlib import Path
from tkinter import font, messagebox, scrolledtext, ttk

import zmq
from blake3 import blake3 as blake3_hasher

from convert_snap import convert_pickle_to_dir

VERSION = "0"


_HASH_CAP = 128 * 1024 * 1024  # 128 MB


def compute_file_hash(path: str) -> str:
    h = blake3_hasher(max_threads=blake3_hasher.AUTO)
    with open(path, "rb") as f:
        h.update(f.read(_HASH_CAP))
    return h.hexdigest()


def get_or_create_cache(pickle_path: str, device_id: int) -> str:

    cache_root = Path.home() / ".snapviewer_cache"
    file_hash = compute_file_hash(pickle_path)
    cache_key = f"{file_hash}_dev{device_id}_v{VERSION}"
    cache_dir = cache_root / cache_key
    alloc_file = cache_dir / "allocations.json"
    db_file = cache_dir / "elements.db"
    if alloc_file.exists() and db_file.exists():
        print("Cache hit:")
        print(f"- version: {VERSION}")
        print(f"- path:    {cache_dir}")
        return str(cache_dir)
    print(f"Cache miss, converting pickle: {pickle_path}")
    cache_dir.mkdir(parents=True, exist_ok=True)
    convert_pickle_to_dir(pickle_path, str(cache_dir), device_id)
    return str(cache_dir)


# Global reference to the app instance for callback access
app_instance = None
sql_client = None
renderer_process = None

HELP_MSG = """Execute any SqLite commands.
Special commands:
    --help: display this help message
    --schema: display database schema of the memory snapshot
    --clear: clear REPL output
    --find <pattern>: find the message panel (on the left) with a pattern.
                      case INsensitive, does NOT support regex
"""
DATABASE_SCHEMA = """CREATE TABLE allocs (
    idx INTEGER PRIMARY KEY,
    size INTEGER,
    start_time INTEGER,
    end_time INTEGER,
    callstack TEXT
);"""


class ZeroMQReceiver(threading.Thread):
    """Background thread that receives messages from renderer via ZeroMQ SUB socket"""

    def __init__(self, host, port, app):
        super().__init__(daemon=True)
        self.host = host
        self.port = port
        self.app = app
        self.context = zmq.Context()
        self.socket = None
        self.running = True
        self.poller = None

    def run(self):
        """Receive messages and update GUI thread-safely"""
        self.socket = self.context.socket(zmq.SUB)
        self.socket.connect(f"tcp://{self.host}:{self.port}")
        self.socket.setsockopt_string(zmq.SUBSCRIBE, "")
        self.poller = zmq.Poller()
        self.poller.register(self.socket, zmq.POLLIN)

        while self.running:
            # Use poll with timeout for clean shutdown
            socks = dict(self.poller.poll(timeout=100))
            if self.socket in socks and socks[self.socket] == zmq.POLLIN:
                try:
                    message = self.socket.recv_string(zmq.NOBLOCK)
                    # Use after() for thread-safe UI updates
                    self.app.root.after(0, self.app.update_message, message)
                except zmq.ZMQError:
                    pass

    def stop(self):
        """Stop the receiver thread"""
        self.running = False
        if self.socket:
            self.socket.close()
        self.context.term()


class ZeroMQSQLClient:
    """Client for sending SQL commands to renderer via ZeroMQ REQ socket"""

    def __init__(self, host, port):
        self.host = host
        self.port = port
        self.context = zmq.Context()
        self.socket = None

    def connect(self):
        """Connect to the renderer's REP socket"""
        self.socket = self.context.socket(zmq.REQ)
        self.socket.connect(f"tcp://{self.host}:{self.port}")

    def execute_sql(self, command: str) -> str:
        """Send SQL command and receive response"""
        if not self.socket:
            return "Error: Not connected to renderer"
        self.socket.send_string(command)
        return self.socket.recv_string()

    def close(self):
        """Close the connection"""
        if self.socket:
            self.socket.close()
        self.context.term()


class MessagePanel(ttk.Frame):
    """Panel that displays messages from main thread"""

    def __init__(self, parent):
        super().__init__(parent)
        self.parent = parent
        self.setup_ui()

    def setup_ui(self):
        """Setup the UI components"""
        # Configure padding
        self.configure(padding="20")

        # Configure fonts - try to use the font file if available
        font_path = os.path.join(os.path.dirname(__file__), "assets", "JetBrainsMono-Medium.ttf")
        try:
            if os.path.exists(font_path):
                # Register the font with tkinter using the low-level tk interface
                self.winfo_toplevel().tk.call(
                    "font", "create", "JetBrainsMonoCustom", "-family", "JetBrains Mono", "-size", "14"
                )
                # Try to load the actual font file using platform-specific methods
                if platform.system() == "Windows":
                    try:
                        # Load font temporarily for this session
                        gdi32 = ctypes.windll.gdi32
                        gdi32.AddFontResourceW.argtypes = [wintypes.LPCWSTR]
                        gdi32.AddFontResourceW.restype = ctypes.c_int
                        result = gdi32.AddFontResourceW(font_path)
                        if result:
                            print(f"Successfully loaded JetBrains Mono font from {font_path}")
                            font_family = "JetBrains Mono"
                        else:
                            raise Exception("AddFontResourceW failed")
                    except Exception as e:
                        print(f"Could not load font via Windows API: {e}")
                        font_family = "Consolas"
                else:
                    # For Unix-like systems, we can't load fonts at runtime easily
                    # Just use the family name and hope it's installed
                    font_family = "JetBrains Mono"
            else:
                raise FileNotFoundError("Font file not found")
        except Exception as e:
            print(f"Font loading failed: {e}")
            # Fallback to family name (works if font is installed system-wide)
            try:
                test_font = font.Font(family="JetBrains Mono", size=12)
                if "JetBrains Mono" in test_font.actual("family"):
                    font_family = "JetBrains Mono"
                else:
                    font_family = "Consolas"
            except Exception as _:
                # Final fallback to monospace
                font_family = "Consolas"

        # Create the actual font objects
        try:
            self.title_font = font.Font(family=font_family, size=20, weight="bold")
            self.mono_font = font.Font(family=font_family, size=14)
        except Exception as _:
            # Ultimate fallback
            self.title_font = font.Font(family="Consolas", size=20, weight="bold")
            self.mono_font = font.Font(family="Consolas", size=14)

        # Title
        title_label = ttk.Label(self, text="Messages", font=self.title_font)
        title_label.configure(foreground="#e91e63")
        title_label.pack(anchor="w", pady=(0, 10))

        # Message display
        self.text_widget = scrolledtext.ScrolledText(
            self,
            font=self.mono_font,
            state="disabled",
            wrap=tk.WORD,
            height=20,
            width=60,
            bg="#fce4ec",
            fg="#2d2d2d",
            selectbackground="#e91e63",
            selectforeground="white",
            highlightthickness=0,  # Remove highlight border
            insertbackground="#e91e63",
            padx=10,  # Added horizontal padding
            pady=10,  # Added vertical padding
        )
        self.text_widget.pack(fill=tk.BOTH, expand=True)

        # Set initial message
        self.update_content("""This panel will show:
- On left click, info of the allocation you left clicked on
- On right click, your current mouse position (x -> timestamp, y -> memory)""")

    def update_content(self, message: str):
        """Update the message content"""
        # Ensure proper Unicode handling
        if isinstance(message, bytes):
            message = message.decode("utf-8", errors="replace")

        self.text_widget.configure(state="normal")
        self.text_widget.delete(1.0, tk.END)
        self.text_widget.insert(1.0, message)
        self.text_widget.configure(state="disabled")


class HistoryEntry(ttk.Entry):
    """Entry widget subclass to handle command history"""

    def __init__(self, parent, **kwargs):
        super().__init__(parent, **kwargs)
        self.command_history = []
        self.history_index = 0
        self.bind("<Up>", self.on_up_arrow)
        self.bind("<Down>", self.on_down_arrow)

    def on_up_arrow(self, event):
        if self.command_history:
            self.history_index = max(0, self.history_index - 1)
            self.delete(0, tk.END)
            self.insert(0, self.command_history[self.history_index])

    def on_down_arrow(self, event):
        if self.command_history:
            self.history_index += 1
            if self.history_index >= len(self.command_history):
                self.history_index = len(self.command_history)
                self.delete(0, tk.END)
            else:
                self.delete(0, tk.END)
                self.insert(0, self.command_history[self.history_index])


class REPLPanel(ttk.Frame):
    """REPL panel with input and scrollable output"""

    REPL_HINT = (
        "SQLite REPL - This is a SQLite database storing the allocation data.",
        "Type `--help` to see available commands.",
        "Type `--find <pattern>` to search messages.",
        "Ctrl+D to quit application.",
    )

    def __init__(self, parent, args):
        super().__init__(parent)
        self.args = args
        self.parent = parent
        self.setup_ui()

    def setup_ui(self):
        """Setup the UI components"""
        # Configure padding
        self.configure(padding="20")

        # Configure fonts - try to use the font file if available
        font_path = os.path.join(os.path.dirname(__file__), "assets", "JetBrainsMono-Medium.ttf")
        try:
            if os.path.exists(font_path):
                # Register the font with tkinter using the low-level tk interface
                self.winfo_toplevel().tk.call(
                    "font", "create", "JetBrainsMonoCustom", "-family", "JetBrains Mono", "-size", "14"
                )
                # Try to load the actual font file using platform-specific methods
                if platform.system() == "Windows":
                    try:
                        # Load font temporarily for this session
                        gdi32 = ctypes.windll.gdi32
                        gdi32.AddFontResourceW.argtypes = [wintypes.LPCWSTR]
                        gdi32.AddFontResourceW.restype = ctypes.c_int
                        result = gdi32.AddFontResourceW(font_path)
                        if result:
                            print(f"Successfully loaded JetBrains Mono font from {font_path}")
                            font_family = "JetBrains Mono"
                        else:
                            raise Exception("AddFontResourceW failed")
                    except Exception as e:
                        print(f"Could not load font via Windows API: {e}")
                        font_family = "Consolas"
                else:
                    # For Unix-like systems, we can't load fonts at runtime easily
                    # Just use the family name and hope it's installed
                    font_family = "JetBrains Mono"
            else:
                raise FileNotFoundError("Font file not found")
        except Exception as e:
            print(f"Font loading failed: {e}")
            # Fallback to family name (works if font is installed system-wide)
            try:
                test_font = font.Font(family="JetBrains Mono", size=12)
                if "JetBrains Mono" in test_font.actual("family"):
                    font_family = "JetBrains Mono"
                else:
                    font_family = "Consolas"
            except Exception as _:
                # Final fallback to monospace
                font_family = "Consolas"

        # Create the actual font objects
        try:
            self.title_font = font.Font(family=font_family, size=20, weight="bold")
            self.mono_font = font.Font(family=font_family, size=14)
        except Exception as _:
            # Ultimate fallback
            self.title_font = font.Font(family="Consolas", size=20, weight="bold")
            self.mono_font = font.Font(family="Consolas", size=14)

        # Title
        title_label = ttk.Label(self, text="SQLite REPL", font=self.title_font)
        title_label.configure(foreground="#e91e63")
        title_label.pack(anchor="w", pady=(0, 10))

        # Output area
        self.output_text = scrolledtext.ScrolledText(
            self,
            font=self.mono_font,
            state="disabled",
            wrap=tk.WORD,
            height=15,
            width=60,
            bg="#fce4ec",
            fg="#2d2d2d",
            selectbackground="#e91e63",
            selectforeground="white",
            highlightthickness=0,  # Remove highlight border
            insertbackground="#e91e63",
            padx=10,  # Added horizontal padding
            pady=10,  # Added vertical padding
        )
        self.output_text.pack(fill=tk.BOTH, expand=True, pady=(0, 10))

        # Input area
        input_frame = ttk.Frame(self)
        input_frame.pack(fill=tk.X, pady=(0, 10))

        prompt_label = ttk.Label(input_frame, text="> ", font=self.mono_font)
        prompt_label.configure(foreground="#e91e63")
        prompt_label.pack(side=tk.LEFT)

        self.input_entry = HistoryEntry(input_frame, font=self.mono_font, width=50)
        self.input_entry.bind("<Return>", self.on_submit)
        self.input_entry.pack(side=tk.LEFT, fill=tk.X, expand=True)

        # Initialize with hint
        self.output_lines = list(REPLPanel.REPL_HINT)
        self.update_output()

        # Focus the input
        self.input_entry.focus_set()

    def on_submit(self, event=None):
        """Handle command submission"""
        command = self.input_entry.get().strip()
        if command:
            history = self.input_entry.command_history
            # Add command to history if not empty and not a duplicate of the last command
            if not history or history[-1] != command:
                history.append(command)
            # After submitting, the history index should point to the "new command" state
            self.input_entry.history_index = len(history)

            if command == "--clear":
                self.output_lines = list(REPLPanel.REPL_HINT)
            else:
                # is input command
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.output_lines.append(f"[{timestamp}] > {command}")
                # split at first whitespace
                cmdlist = command.split(None, 1)
                cmd = cmdlist[0]
                pattern = cmdlist[1] if len(cmdlist) > 1 else None
                if cmd == "--find":
                    if not pattern:
                        self.output_lines.append(f"[{timestamp}]\nUsage: --find <pattern>")
                    else:
                        global app_instance
                        if app_instance and hasattr(app_instance, "message_panel"):
                            message_content = app_instance.message_panel.text_widget.get("1.0", tk.END)
                            lines = message_content.splitlines()
                            found_lines = [line for line in lines if pattern.lower() in line.lower()]

                            if found_lines:
                                result = (
                                    f"Found {len(found_lines)} matching lines for '{pattern}':\n"
                                    + "\n".join(found_lines)
                                )
                                self.output_lines.append(f"[{timestamp}]\n{result}")
                            else:
                                self.output_lines.append(f"[{timestamp}]\nNo matches found for '{pattern}'.")
                        else:
                            self.output_lines.append(f"[{timestamp}]\nError: Could not access message panel.")
                elif cmd == "--help":
                    self.output_lines.append(f"[{timestamp}]\n{HELP_MSG}")
                elif cmd == "--schema":
                    self.output_lines.append(f"[{timestamp}]\n{DATABASE_SCHEMA}")
                else:
                    output = app_instance.sql_client.execute_sql(command)
                    self.output_lines.append(f"[{timestamp}]\n{output}")

            self.update_output()

        # Clear input
        self.input_entry.delete(0, tk.END)

    def update_output(self):
        """Update the output display"""
        output_content = "\n".join(self.output_lines)
        # Ensure proper Unicode handling
        if isinstance(output_content, bytes):
            output_content = output_content.decode("utf-8", errors="replace")

        self.output_text.configure(state="normal")
        self.output_text.delete(1.0, tk.END)
        self.output_text.insert(1.0, output_content)
        self.output_text.configure(state="disabled")
        # Auto-scroll to bottom
        self.output_text.see(tk.END)


class SnapViewerApp:
    """Main GUI application with ZeroMQ communication"""

    def __init__(self, args, sql_client):
        self.args = args
        self.sql_client = sql_client
        self.root = tk.Tk()
        self.receiver = None
        self.setup_ui(args.dir)
        self.start_receiver(args.pub_port)

    def start_receiver(self, pub_port):
        """Start the ZeroMQ receiver thread"""
        self.receiver = ZeroMQReceiver("127.0.0.1", pub_port, self)
        self.receiver.start()

    def setup_ui(self, path: str):
        """Setup the main UI"""
        self.root.title(f"SnapViewer - Memory Allocation Viewer & SQLite REPL ( Path: {path} )")
        self.root.geometry("1600x1200")

        # Configure colors and styling
        self.root.configure(bg="#fff5f8")

        # Configure style for ttk widgets
        style = ttk.Style()
        style.theme_use("clam")

        # Configure ttk styles to match the PyQt6 appearance
        style.configure(
            "Title.TLabel", foreground="#e91e63", background="#fff5f8", font=("JetBrains Mono", 20, "bold")
        )

        # Remove the border from the Panel.TFrame style
        style.configure("Panel.TFrame", background="#f8bbdd", relief="flat", borderwidth=0)

        # Create main container
        main_frame = ttk.Frame(self.root, padding="20")
        main_frame.pack(fill=tk.BOTH, expand=True)

        # Create panels
        self.message_panel = MessagePanel(main_frame)
        self.repl_panel = REPLPanel(main_frame, self.args)

        # Configure panel styling
        self.message_panel.configure(style="Panel.TFrame")
        self.repl_panel.configure(style="Panel.TFrame")

        # Pack panels side by side
        self.message_panel.pack(side=tk.LEFT, fill=tk.BOTH, expand=True, padx=(0, 10))

        # Add separator
        separator = ttk.Separator(main_frame, orient="vertical")
        separator.pack(side=tk.LEFT, fill=tk.Y, padx=10)

        self.repl_panel.pack(side=tk.RIGHT, fill=tk.BOTH, expand=True, padx=(10, 0))

        # Add keyboard shortcuts
        self.root.bind("<Control-d>", lambda e: self.close())
        self.root.bind("<Control-q>", lambda e: self.close())

        # Handle window close event
        self.root.protocol("WM_DELETE_WINDOW", self.close)

    def update_message(self, message: str):
        """Update the message panel content"""
        self.message_panel.update_content(message)

    def close(self):
        """Handle window close event"""
        result = messagebox.askyesno(
            "Quit SnapViewer", "Are you sure you want to quit SnapViewer?", default=messagebox.NO
        )

        if result:
            # Stop the receiver thread
            if self.receiver:
                self.receiver.stop()
            self.root.quit()
            self.root.destroy()
            # Terminate the application
            terminate()

    def run(self):
        """Start the GUI event loop"""
        self.root.mainloop()


def terminate():
    """Terminate the application"""
    global renderer_process, sql_client

    # Close SQL client
    if sql_client:
        sql_client.close()

    # Terminate renderer process
    if renderer_process:
        renderer_process.terminate()
        renderer_process.wait()

    os._exit(0)


def spawn_renderer(args):
    """Spawn the renderer process"""
    global renderer_process

    if args.bin:
        renderer_binary = args.bin
        if not Path(renderer_binary).exists():
            print(f"Error: Renderer binary not found at {renderer_binary}")
            sys.exit(1)
    else:
        # Find the renderer binary
        # First try the target/release directory
        # Get the directory where this script is located
        script_dir = Path(__file__).parent
        exe_suffix = ".exe" if platform.system() == "Windows" else ""
        renderer_paths = [
            script_dir / "target" / "release" / f"snapviewer-renderer{exe_suffix}",
            script_dir / "target" / "debug" / f"snapviewer-renderer{exe_suffix}",
        ]

        renderer_binary = None
        for path in renderer_paths:
            if path.exists():
                renderer_binary = str(path)
                break

        if not renderer_binary:
            # Try to find via cargo
            print("Renderer binary not found in expected locations, building...")
            subprocess.run(
                ["cargo", "build", "--release", "--bin", "snapviewer-renderer"],
                cwd=script_dir,
                check=True,
            )
            renderer_binary = str(script_dir / "target" / "release" / f"snapviewer-renderer{exe_suffix}")

    cmd = [
        renderer_binary,
        "--dir",
        args.dir,
        "--res",
        str(args.resolution[0]),
        str(args.resolution[1]),
        "--resolution-ratio",
        str(args.resolution_ratio),
        "--pub-port",
        str(args.pub_port),
        "--rep-port",
        str(args.rep_port),
        "--log",
        args.log,
    ]

    print(f"Starting renderer process: {' '.join(cmd)}")
    renderer_process = subprocess.Popen(cmd)


def run_gui(args):
    """Run the GUI application"""
    global app_instance, sql_client

    # Create SQL client and connect
    sql_client = ZeroMQSQLClient("127.0.0.1", args.rep_port)
    sql_client.connect()

    app_instance = SnapViewerApp(args, sql_client)
    app_instance.run()

    print("Stopping SnapViewer application...")
    terminate()


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Python GUI with Message Display Area and SQLite REPL")

    def positive_int(value):
        ivalue = int(value)
        if ivalue <= 0:
            raise argparse.ArgumentTypeError(f"'{value}' is an invalid positive int value")
        return ivalue

    parser.add_argument(
        "--bin",
        type=str,
        default=None,
        help="Path to the renderer binary. Skips auto-detection and cargo build fallback.",
    )
    parser.add_argument(
        "--log",
        type=str,
        choices=["info", "trace"],
        default="info",
        help="Set the logging level (info or trace).",
    )
    parser.add_argument(
        "--res",
        type=positive_int,
        nargs=2,  # Expect exactly 2 arguments for resolution
        default=[2400, 1000],  # Default as a list
        metavar=("WIDTH", "HEIGHT"),  # Help text for the arguments
        help="Specify resolution as two positive integers (WIDTH HEIGHT).",
    )
    parser.add_argument(
        "--pub-port",
        type=int,
        default=5555,
        help="ZeroMQ PUB socket port (Renderer -> UI). Default: 5555",
    )
    parser.add_argument(
        "--rep-port",
        type=int,
        default=5556,
        help="ZeroMQ REP socket port (UI -> Renderer). Default: 5556",
    )
    parser.add_argument(
        "-rr",
        "--resolution-ratio",
        type=float,
        default=1.0,
        help="Resolution ratio for high-DPI displays (e.g., 2.0 for Retina). Default: 1.0",
    )

    source_group = parser.add_mutually_exclusive_group(required=True)
    source_group.add_argument(
        "-d",
        "--dir",
        type=str,
        help="Directory containing allocations.json and elements.db",
    )
    source_group.add_argument(
        "--pickle",
        type=str,
        help="Path to a .pickle snapshot. Preprocessing result is cached under ~/.snapviewer_cache/",
    )

    parser.add_argument(
        "--device",
        type=int,
        default=0,
        help="Device ID to use when --pickle is provided. Default: 0",
    )

    args = parser.parse_args()

    # Convert the resolution list to a tuple after parsing
    args.resolution = tuple(args.res)

    if args.pickle:
        if not os.path.exists(args.pickle):
            print(f"Error: pickle file '{args.pickle}' does not exist.")
            exit(1)
        args.dir = get_or_create_cache(args.pickle, args.device)

    # Verify that the path exists
    if not os.path.exists(args.dir):
        print(f"Error: The specified path '{args.dir}' does not exist.")
        exit(1)  # Exit the program with an error code

    # Check ports are available before spawning anything
    import socket

    for port, name in [(args.pub_port, "pub"), (args.rep_port, "rep")]:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            if s.connect_ex(("127.0.0.1", port)) == 0:
                print(f"Error: port {port} (--{name}-port) is already in use.")
                exit(1)

    # Spawn the renderer process
    spawn_renderer(args)

    # Give the renderer a moment to start up and bind its sockets
    time.sleep(0.5)

    # Run the GUI (this is now the main process)
    run_gui(args)


if __name__ == "__main__":
    main()
