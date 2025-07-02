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


class LoggingWidget(ScrollableContainer):
    """Scrollable logging widget that adds entries every 0.3 seconds"""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.log_lines = []
        self.counter = 0
        self.focused = False

    def compose(self) -> ComposeResult:
        yield Static("", id="log_content")

    def on_mount(self) -> None:
        """Start logging when widget is mounted"""
        self.add_log_entry()  # Add first entry immediately

    def add_log_entry(self) -> None:
        """Add a new log entry"""
        self.counter += 1
        timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
        log_entry = f"[{timestamp}] Log entry #{self.counter}"
        self.log_lines.append(log_entry)

        # Keep only last 1000 entries to prevent memory issues
        if len(self.log_lines) > 1000:
            self.log_lines = self.log_lines[-1000:]

        self.query_one("#log_content").update("\n".join(self.log_lines))

        # Auto-scroll to bottom
        # self.scroll_end(animate=False)

        # Schedule next log entry
        self.set_timer(0.3, self.add_log_entry)

    def on_focus(self) -> None:
        self.focused = True
        self.add_class("focused")

    def on_blur(self) -> None:
        self.focused = False
        self.remove_class("focused")


class REPLWidget(Vertical):
    """REPL widget with input and scrollable output"""

    def __init__(self, **kwargs):
        super().__init__(**kwargs)
        self.output_lines = ["[SqLite REPL] Type `--help` to see commands"]
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

    def compose(self) -> ComposeResult:
        """Create the UI layout"""
        with Horizontal():
            yield LoggingWidget(id="logging_panel")
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
