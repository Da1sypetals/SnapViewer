#!/usr/bin/env python3
"""
SnapViewer GUI using TKinter
Features:
- Left panel: Displays messages from main thread
- Right panel: Echo REPL
- Main thread runs viewer() function
- Thread communication via TKinter thread-safe methods
"""

import argparse
import os
import threading
from datetime import datetime
import tkinter as tk
from tkinter import ttk, messagebox, scrolledtext, font

from snapviewer import SnapViewer

# Global reference to the app instance for callback access
app_instance = None
snapviewer = None


def message_callback(message: str):
    """Callback that updates GUI via thread-safe method"""
    global app_instance
    if app_instance:
        # Use after() for thread-safe UI updates
        app_instance.root.after(0, app_instance.update_message, message)


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
                import platform

                if platform.system() == "Windows":
                    try:
                        import ctypes
                        from ctypes import wintypes

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
- On right click, your current mouse position (x -> timestamp, y -> memory)

Welcome to SnapViewer! Ready to explore your data.""")

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

    REPL_HINT = [
        "SQLite REPL - This is a SQLite database storing the allocation data.",
        "Type `--help` to see available commands",
        "Ctrl+D to quit application",
        "Ready for your queries!",
    ]

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
                import platform

                if platform.system() == "Windows":
                    try:
                        import ctypes
                        from ctypes import wintypes

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
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.output_lines.append(f"[{timestamp}] > {command}")
                try:
                    output = snapviewer.execute_sql(command)
                    self.output_lines.append(f"[{timestamp}]\n{output}")
                except Exception as e:
                    self.output_lines.append(f"[{timestamp}]\nError: {e}")

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
    """Main GUI application with thread communication"""

    def __init__(self, args):
        self.args = args
        self.root = tk.Tk()
        self.setup_ui()

    def setup_ui(self):
        """Setup the main UI"""
        self.root.title("SnapViewer - Memory Allocation Viewer & SQLite REPL")
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
            self.root.quit()
            self.root.destroy()
            # Terminate the application
            terminate()

    def run(self):
        """Start the GUI event loop"""
        self.root.mainloop()


def terminate():
    """Terminate the application"""
    os._exit(0)


def run_gui(args):
    """Run the GUI application in a separate thread"""
    global app_instance

    app_instance = SnapViewerApp(args)
    app_instance.run()

    print("Stopping SnapViewer application...")
    terminate()


def main():
    """Run the application"""
    parser = argparse.ArgumentParser(description="Python GUI with Message Display Area and Echo REPL")

    def positive_int(value):
        ivalue = int(value)
        if ivalue <= 0:
            raise argparse.ArgumentTypeError(f"'{value}' is an invalid positive int value")
        return ivalue

    parser.add_argument(
        "--log",
        type=str,
        choices=["info", "trace"],
        default="info",
        help="Set the logging level (info or trace).",
    )
    parser.add_argument(
        "-p",
        "--path",
        type=str,
        required=True,
        help="Path to the trace file.",
    )
    parser.add_argument(
        "--res",
        type=positive_int,
        nargs=2,  # Expect exactly 2 arguments for resolution
        default=[2400, 1000],  # Default as a list
        metavar=("WIDTH", "HEIGHT"),  # Help text for the arguments
        help="Specify resolution as two positive integers (WIDTH HEIGHT).",
    )

    args = parser.parse_args()

    # Convert the resolution list to a tuple after parsing
    args.resolution = tuple(args.res)

    # Verify that the path exists
    if not os.path.exists(args.path):
        print(f"Error: The specified path '{args.path}' does not exist.")
        exit(1)  # Exit the program with an error code

    global snapviewer
    snapviewer = SnapViewer(args.path, args.resolution, args.log)

    # Start GUI in a separate thread (non-daemon so it stays alive)
    gui_thread = threading.Thread(target=run_gui, args=(args,), daemon=True)
    gui_thread.start()

    # Run viewer in main thread (blocking infinite loop)
    try:
        # this calls into Rust extension.
        # block current thread, but does NOT hold GIL.
        # MUST run on main thread.
        snapviewer.viewer(message_callback)
    except KeyboardInterrupt:
        print("\nShutting down...")
        # The GUI close event will handle termination
    except Exception as e:
        print(f"Error in viewer: {e}")
        terminate()


if __name__ == "__main__":
    main()
