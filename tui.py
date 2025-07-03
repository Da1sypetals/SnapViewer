#!/usr/bin/env python3
"""
Python TUI with Message Display Area and Echo REPL
Features:
- Left panel: Displays messages from subprocess
- Right panel: Echo REPL
- Subprocess runs viewer() function
- Inter-process communication via shared object
"""

import os
import multiprocessing
from datetime import datetime
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import Static, Input
from textual.containers import ScrollableContainer
from textual.binding import Binding
from textual import events
from snapviewer import sql_repl, execute_sql, viewer
import argparse


# args = None # REMOVE THIS GLOBAL DECLARATION


def subprocess_worker(shared_state, args_for_subprocess):
    """Worker function that runs in subprocess"""

    def message_callback(message: str):
        """Callback that updates shared state with new message"""
        with shared_state.get_lock():
            shared_state.value = message.encode("utf-8")

    # Run viewer with the callback in the subprocess main thread
    viewer(
        message_callback,
        args_for_subprocess.path,
        args_for_subprocess.resolution,
        args_for_subprocess.log,
    )


class MessageWidget(ScrollableContainer):
    """Widget that displays messages from subprocess"""

    def __init__(self, args, **kwargs):
        super().__init__(**kwargs)
        self.args = args  # Store args as an instance attribute
        self.current_message = "Hello!"
        self.focused = False
        self.shared_state = None
        self.subprocess_process = None

    def compose(self) -> ComposeResult:
        yield Static(self.current_message, id="log_content")

    def on_mount(self) -> None:
        """Start subprocess and message checking"""
        self.start_subprocess()
        self.set_interval(0.1, self.check_messages)

    def start_subprocess(self) -> None:
        """Start the subprocess with viewer"""
        # Create shared object for inter-process communication
        # Using Array with lock for thread-safe access
        self.shared_state = multiprocessing.Array("c", 1048576)

        # Initialize with default message
        with self.shared_state.get_lock():
            self.shared_state.value = self.current_message.encode("utf-8")

        # Start subprocess
        self.subprocess_process = multiprocessing.Process(
            target=subprocess_worker,
            args=(self.shared_state, self.args),  # Pass args to subprocess_worker
        )
        self.subprocess_process.daemon = True
        self.subprocess_process.start()

    def check_messages(self) -> None:
        """Check for message updates from subprocess at fixed interval"""
        if self.shared_state is not None:
            try:
                with self.shared_state.get_lock():
                    new_message = self.shared_state.value.decode("utf-8").rstrip("\x00")
                    if new_message != self.current_message:
                        self.current_message = new_message
            except Exception:
                pass  # Handle decoding errors gracefully

        # Update log content
        self.query_one("#log_content").update(self.current_message)

    def on_focus(self) -> None:
        self.focused = True
        self.add_class("focused")

    def on_blur(self) -> None:
        self.focused = False
        self.remove_class("focused")

    def cleanup(self) -> None:
        """Clean up subprocess"""
        if self.subprocess_process and self.subprocess_process.is_alive():
            self.subprocess_process.terminate()
            self.subprocess_process.join(timeout=1.0)


class REPLWidget(Vertical):
    """REPL widget with input and scrollable output"""

    def __init__(self, args, **kwargs):
        super().__init__(**kwargs)
        self.args = args  # Store args as an instance attribute
        self.output_lines = ["[SqLite REPL] Type `--help` to see commands"]
        self.focused = False
        self.dbptr = sql_repl(self.args.path, self.args.log)

    def compose(self) -> ComposeResult:
        with ScrollableContainer(id="repl_output"):
            yield Static("\n".join(self.output_lines), id="repl_content")
        yield Input(placeholder="Type a command...", id="repl_input")

    def on_mount(self) -> None:
        """Focus the input when mounted"""
        self.query_one("#repl_input").focus()

    def on_input_submitted(self, event: Input.Submitted) -> None:
        """Handle command submission"""
        command = event.value.strip()
        if command:
            timestamp = datetime.now().strftime("%H:%M:%S")
            self.output_lines.append(f"[{timestamp}] > {command}")
            output = execute_sql(self.dbptr, command)
            self.output_lines.append(f"[{timestamp}] {output}")

            self.query_one("#repl_content").update("\n".join(self.output_lines))

            # Auto-scroll to bottom
            scroll_container = self.query_one("#repl_output")
            scroll_container.scroll_end(animate=False)

        # Clear input
        event.input.value = ""

    def on_focus(self) -> None:
        self.focused = True
        self.add_class("focused")
        self.query_one("#repl_input").focus()

    def on_blur(self) -> None:
        self.focused = False
        self.remove_class("focused")


class SnapViewerApp(App):
    """Main TUI application with subprocess communication"""

    CSS = """
    Screen {
        layout: horizontal;
    }
    
    #message_panel {
        width: 50%;
        border: solid $primary;
        margin: 1;
        padding: 1;
    }
    
    #repl_panel {
        width: 50%;
        border: solid $primary;
        margin: 1;
        padding: 1;
    }
    
    #log_content {
        height: auto;
    }
    
    #repl_output {
        height: 1fr;
        margin-bottom: 1;
    }
    
    #repl_content {
        height: auto;
    }
    
    #repl_input {
        dock: bottom;
    }
    
    Static {
        color: $text;
    }
    """

    BINDINGS = [
        Binding("ctrl+d", "quit", "Quit", show=True),
        Binding("tab", "focus_next", "Next Focus", show=False),
        Binding("shift+tab", "focus_previous", "Previous Focus", show=False),
    ]

    def __init__(self, args):
        super().__init__()
        self.args = args  # Store args as an instance attribute
        self.title = "SnapViewer Viewer and SqLite REPL"
        self.sub_title = "Ctrl+D to quit, Click to focus, Tab to switch"

    def compose(self) -> ComposeResult:
        """Create the UI layout"""
        with Horizontal():
            yield MessageWidget(self.args, id="message_panel")  # Pass args to MessageWidget
            yield REPLWidget(self.args, id="repl_panel")  # Pass args to REPLWidget

    def on_mount(self) -> None:
        """Initialize focus on the REPL"""
        self.query_one("#repl_panel").focus()

    def on_click(self, event: events.Click) -> None:
        """Handle mouse clicks to switch focus"""
        widget, _ = self.get_widget_at(*event.screen_offset)

        if widget:
            for ancestor in [widget] + list(widget.ancestors):
                if ancestor.id in ["message_panel", "repl_panel"]:
                    ancestor.focus()
                    break

    def action_quit(self) -> None:
        """Quit the application and cleanup subprocess"""
        # Clean up subprocess before quitting
        message_widget = self.query_one("#message_panel")
        message_widget.cleanup()
        self.exit()


def main():
    """Run the application"""
    # global args # REMOVE THIS LINE

    parser = argparse.ArgumentParser(description="Python TUI with Message Display Area and Echo REPL")

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

    # Required for multiprocessing on some platforms
    multiprocessing.set_start_method("spawn", force=True)

    app = SnapViewerApp(args)  # Pass args to the app constructor
    app.run()


if __name__ == "__main__":
    main()
