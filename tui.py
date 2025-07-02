#!/usr/bin/env python3
"""
Python TUI with Logging Area and Echo REPL
Features:
- Left panel: Continuous logging every 0.3 seconds
- Right panel: Echo REPL
- Both panels are scrollable
- Mouse click to switch focus
- Ctrl+D to quit
"""

import os
from datetime import datetime
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import Static, Input
from textual.containers import ScrollableContainer
from textual.binding import Binding
from textual import events
from snapviewer import sql_repl, execute_sql, viewer
import queue
import threading


class LoggingWidget(ScrollableContainer):
    """
    A scrollable widget designed to display log entries.
    It receives log entries from an external queue (populated by the Rust extension).
    """

    def __init__(self, log_queue: queue.Queue, **kwargs):
        """
        Initializes the LoggingWidget.

        Args:
            log_queue (queue.Queue): The queue from which to read log entries.
        """
        super().__init__(**kwargs)
        self.log_lines = []
        self.log_queue = log_queue
        self.focused = False

    def compose(self) -> ComposeResult:
        """Composes the child widgets for this container."""
        yield Static("", id="log_content")

    def on_mount(self) -> None:
        """
        Called when the widget is mounted in the DOM.
        Starts a periodic check of the log queue.
        """
        def rust_callback(log_entry: str):
                self.call_from_thread(self.log_queue.put, log_entry)

            # This is CRUCIAL: The call to start_rust_logging MUST be in a separate
            # Python thread to prevent blocking the Textual UI.
        threading.Thread(target=viewer, args=(rust_callback,str(os.path.join("..", "snapshots", "8spattn.zip")), (2400, 1000), "info"), daemon=True).start()
        self.set_interval(0.03, self.check_log_queue)

    def check_log_queue(self) -> None:
        """
        Checks the internal queue for new log entries and adds them to the display.
        This method runs on the main Textual UI thread.
        """
        # Process all available items in the queue to avoid backlog.
        while not self.log_queue.empty():
            try:
                # Get a log entry without blocking.
                log_entry = self.log_queue.get_nowait()
                self.add_log_entry(log_entry)
            except queue.Empty:
                # If the queue becomes empty during the loop, break.
                break

    def add_log_entry(self, log_entry: str) -> None:
        """
        Adds a new log entry to the list and updates the Static widget.

        Args:
            log_entry (str): The log string to add.
        """
        self.log_lines.append(log_entry)

        # Keep only the last 1000 entries to manage memory usage
        # if len(self.log_lines) > 1000:
        #     self.log_lines = self.log_lines[-1000:]

        # Update the content of the Static widget.
        self.query_one("#log_content", Static).update("\n".join(self.log_lines))

        # Auto-scroll to the bottom to show the latest entries.
        self.scroll_end(animate=False)

    def on_focus(self) -> None:
        """Handles when the widget gains focus."""
        self.focused = True
        self.add_class("focused")

    def on_blur(self) -> None:
        """Handles when the widget loses focus."""
        self.focused = False
        self.remove_class("focused")


class REPLWidget(Vertical):
    """REPL widget with input and scrollable output"""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.output_lines = ["Echo REPL - Type commands and press Enter"]
        self.focused = False
        self.dbptr = sql_repl(os.path.join("..", "snapshots", "8spattn.zip"), "info")

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

            # Keep only last 500 entries
            # if len(self.output_lines) > 500:
            #     self.output_lines = self.output_lines[-500:]

            self.query_one("#repl_content").update("\n".join(self.output_lines))

            # Auto-scroll to bottom
            scroll_container = self.query_one("#repl_output")
            scroll_container.scroll_end(animate=False)

        # Clear input
        event.input.value = ""

    def on_focus(self) -> None:
        self.focused = True
        self.add_class("focused")
        # Focus the input when the widget gets focus
        self.query_one("#repl_input").focus()

    def on_blur(self) -> None:
        self.focused = False
        self.remove_class("focused")


class LoggingREPLApp(App):
    """Main TUI application"""

    CSS = """
    Screen {
        layout: horizontal;
    }
    
    #logging_panel {
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
    
    .focused {
        border: solid $accent;
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

    def __init__(self):
        super().__init__()
        self.title = "Python TUI - Logging & REPL"
        self.sub_title = "Ctrl+D to quit, Click to focus, Tab to switch"
        self.log_queue = queue.Queue()

    def compose(self) -> ComposeResult:
        """Create the UI layout"""
        with Horizontal():
            yield LoggingWidget(self.log_queue, id="logging_panel")
            yield REPLWidget(id="repl_panel")

    def on_mount(self) -> None:
        """Initialize focus on the REPL"""
        self.query_one("#repl_panel").focus()

    def on_click(self, event: events.Click) -> None:
        """Handle mouse clicks to switch focus"""
        # Get the widget under the mouse cursor
        widget, _ = self.get_widget_at(*event.screen_offset)

        if widget:
            # Find the parent panel
            for ancestor in [widget] + list(widget.ancestors):
                if ancestor.id in ["logging_panel", "repl_panel"]:
                    ancestor.focus()
                    break

    def action_quit(self) -> None:
        """Quit the application"""
        self.exit()


def main():
    """Run the application"""
    app = LoggingREPLApp()
    app.run()


if __name__ == "__main__":
    main()
