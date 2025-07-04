#!/usr/bin/env python3
"""
Python TUI with Message Display Area and Echo REPL
Features:
- Left panel: Displays messages from main thread
- Right panel: Echo REPL
- Main thread runs viewer() function
- Thread communication via direct callback to TUI
"""

import os
import threading
from datetime import datetime
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import Static, Input
from textual.containers import ScrollableContainer
from textual.binding import Binding
from textual import events
from snapviewer import sql_repl, execute_sql, viewer
import argparse
import signal

# Global reference to the app instance for callback access
app_instance = None


def message_callback(message: str):
    """Callback that updates TUI directly"""
    global app_instance
    if app_instance:
        # Use call_from_thread for thread-safe UI updates
        app_instance.call_from_thread(app_instance.update_message, message)


class MessageWidget(ScrollableContainer):
    """Widget that displays messages from main thread"""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.current_message = """This panel will show:
- On left click, info of the allocation you left clicked on;
- On right click, your current mouse position (x -> timestamp, y -> memory)."""
        self.focused = False

    def compose(self) -> ComposeResult:
        yield Static(self.current_message, id="log_content")

    def update_content(self, message: str):
        """Update the message content"""
        self.current_message = message
        self.query_one("#log_content").update(self.current_message)

    def on_focus(self) -> None:
        self.focused = True
        self.add_class("focused")

    def on_blur(self) -> None:
        self.focused = False
        self.remove_class("focused")


class REPLWidget(Vertical):
    """REPL widget with input and scrollable output"""

    REPL_HINT = (
        "<SqLite REPL> This is a SqLite database storing the allocation data.",
        "Type `--help` to see commands",
        "Ctrl+D to quit.",
    )

    def __init__(self, args, **kwargs):
        super().__init__(**kwargs)
        self.args = args  # Store args as an instance attribute
        self.output_lines = list(REPLWidget.REPL_HINT)
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
            if command == "--clear":
                self.output_lines = list(REPLWidget.REPL_HINT)
            else:
                timestamp = datetime.now().strftime("%H:%M:%S")
                self.output_lines.append(f"[{timestamp}] > {command}")
                output = execute_sql(self.dbptr, command)
                self.output_lines.append(f"[{timestamp}]\n{output}")

            # update REPL content
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
    """Main TUI application with thread communication"""

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
            yield MessageWidget(id="message_panel")
            yield REPLWidget(self.args, id="repl_panel")

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

    def update_message(self, message: str):
        """Update the message panel content (called from callback)"""
        message_widget = self.query_one("#message_panel")
        message_widget.update_content(message)

    def action_quit(self) -> None:
        """Quit the application"""
        self.exit()


def run_tui(args):
    """Run the TUI application in a separate thread"""
    global app_instance
    app_instance = SnapViewerApp(args)
    app_instance.run()

    print("Stopping SnapViewer application...")
    pid = os.getpid()
    os.kill(pid, signal.SIGTERM)


def main():
    """Run the application"""
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

    # Start TUI in a separate thread (non-daemon so it stays alive)
    tui_thread = threading.Thread(target=run_tui, args=(args,))
    tui_thread.start()

    # Run viewer in main thread (blocking infinite loop)
    try:
        viewer(
            message_callback,
            args.path,
            args.resolution,
            args.log,
        )
    except KeyboardInterrupt:
        print("\nShutting down...")
    except Exception as e:
        print(f"Error in viewer: {e}")


if __name__ == "__main__":
    main()
