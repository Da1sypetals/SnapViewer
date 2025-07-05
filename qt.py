#!/usr/bin/env python3
"""
Python GUI with Message Display Area and Echo REPL using PyQt6
Features:
- Left panel: Displays messages from main thread
- Right panel: Echo REPL
- Main thread runs viewer() function
- Thread communication via PyQt Signals and Slots
"""

import argparse
import os
import sys
import threading
from datetime import datetime

from PyQt6.QtCore import Qt, pyqtSignal, QObject
from PyQt6.QtGui import QFont, QShortcut, QKeySequence
from PyQt6.QtWidgets import (
    QApplication,
    QMainWindow,
    QWidget,
    QHBoxLayout,
    QVBoxLayout,
    QLabel,
    QTextEdit,
    QLineEdit,
    QMessageBox,
    QFrame,
)

from snapviewer import SnapViewer

# Global reference to the app instance for callback access
app_instance = None
snapviewer = None


class Communicate(QObject):
    """Signal object for thread-safe communication"""

    message_received = pyqtSignal(str)


def message_callback(message: str):
    """Callback that updates GUI via signals"""
    global app_instance
    if app_instance and app_instance.comm:
        # Emit signal for thread-safe UI updates
        app_instance.comm.message_received.emit(message)


class MessagePanel(QWidget):
    """Panel that displays messages from main thread"""

    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)

        # Configure fonts
        self.title_font = QFont("DejaVu Sans", 14, QFont.Weight.Bold)
        self.mono_font = QFont("DejaVu Sans Mono", 14)

        # Title
        title_label = QLabel("Messages")
        title_label.setFont(self.title_font)
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        layout.addWidget(title_label)

        # Message display
        self.text_widget = QTextEdit()
        self.text_widget.setFont(self.mono_font)
        self.text_widget.setReadOnly(True)
        layout.addWidget(self.text_widget)

        # Set initial message
        self.update_content("""This panel will show:
- On left click, info of the allocation you left clicked on;
- On right click, your current mouse position (x -> timestamp, y -> memory).""")

    def update_content(self, message: str):
        """Update the message content"""
        # Ensure proper Unicode handling
        if isinstance(message, bytes):
            message = message.decode("utf-8", errors="replace")
        self.text_widget.setPlainText(message)
        # Auto-scroll to bottom
        # self.text_widget.verticalScrollBar().setValue(self.text_widget.verticalScrollBar().maximum())


class HistoryLineEdit(QLineEdit):
    """QLineEdit subclass to handle command history"""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.command_history = []
        self.history_index = 0

    def keyPressEvent(self, event):
        if event.key() == Qt.Key.Key_Up:
            self.on_up_arrow()
        elif event.key() == Qt.Key.Key_Down:
            self.on_down_arrow()
        else:
            super().keyPressEvent(event)

    def on_up_arrow(self):
        if self.command_history:
            self.history_index = max(0, self.history_index - 1)
            self.setText(self.command_history[self.history_index])

    def on_down_arrow(self):
        if self.command_history:
            self.history_index += 1
            if self.history_index >= len(self.command_history):
                self.history_index = len(self.command_history)
                self.clear()
            else:
                self.setText(self.command_history[self.history_index])


class REPLPanel(QWidget):
    """REPL panel with input and scrollable output"""

    REPL_HINT = [
        "<SqLite REPL> This is a SqLite database storing the allocation data.",
        "Type `--help` to see commands",
        "Ctrl+D to quit.",
    ]

    def __init__(self, args, parent=None):
        super().__init__(parent)
        self.args = args
        layout = QVBoxLayout(self)
        layout.setContentsMargins(10, 10, 10, 10)

        # Configure fonts
        self.title_font = QFont("DejaVu Sans", 14, QFont.Weight.Bold)
        self.mono_font = QFont("DejaVu Sans Mono", 14)

        # Title
        title_label = QLabel("SQLite REPL")
        title_label.setFont(self.title_font)
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        layout.addWidget(title_label)

        # Output area
        self.output_text = QTextEdit()
        self.output_text.setFont(self.mono_font)
        self.output_text.setReadOnly(True)
        layout.addWidget(self.output_text)

        # Input area
        input_frame = QWidget()
        input_layout = QHBoxLayout(input_frame)
        input_layout.setContentsMargins(0, 0, 0, 0)

        input_layout.addWidget(QLabel("> "))
        self.input_entry = HistoryLineEdit()
        self.input_entry.setFont(self.mono_font)
        self.input_entry.returnPressed.connect(self.on_submit)
        input_layout.addWidget(self.input_entry)
        layout.addWidget(input_frame)

        # Initialize with hint
        self.output_lines = list(REPLPanel.REPL_HINT)
        self.update_output()

        # Focus the input
        self.input_entry.setFocus()

    def on_submit(self):
        """Handle command submission"""
        command = self.input_entry.text().strip()
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
        self.input_entry.clear()

    def update_output(self):
        """Update the output display"""
        output_content = "\n".join(self.output_lines)
        # Ensure proper Unicode handling
        if isinstance(output_content, bytes):
            output_content = output_content.decode("utf-8", errors="replace")
        self.output_text.setPlainText(output_content)
        # Auto-scroll to bottom
        self.output_text.verticalScrollBar().setValue(self.output_text.verticalScrollBar().maximum())


class SnapViewerApp(QMainWindow):
    """Main GUI application with thread communication"""

    def __init__(self, args):
        super().__init__()
        self.args = args
        self.setWindowTitle("SnapViewer Viewer and SQLite REPL")
        self.setGeometry(100, 100, 2400, 1200)

        # Set up communication object for signals
        self.comm = Communicate()
        self.comm.message_received.connect(self.update_message)

        # Create main container
        main_widget = QWidget()
        self.setCentralWidget(main_widget)
        main_layout = QHBoxLayout(main_widget)

        # Create panels
        self.message_panel = MessagePanel()
        self.repl_panel = REPLPanel(args)

        main_layout.addWidget(self.message_panel, 1)

        # Add separator
        separator = QFrame()
        separator.setFrameShape(QFrame.Shape.VLine)
        separator.setFrameShadow(QFrame.Shadow.Sunken)
        main_layout.addWidget(separator)

        main_layout.addWidget(self.repl_panel, 1)

        # Add keyboard shortcuts
        QShortcut(QKeySequence("Ctrl+D"), self, self.close)
        QShortcut(QKeySequence("Ctrl+Q"), self, self.close)

    def update_message(self, message: str):
        """Update the message panel content (slot for signal)"""
        self.message_panel.update_content(message)

    def closeEvent(self, event):
        """Handle window close event"""
        reply = QMessageBox.question(
            self,
            "Quit",
            "Do you want to quit?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply == QMessageBox.StandardButton.Yes:
            event.accept()
            # QApplication.quit() is called automatically, which stops the event loop.
            # We then need to terminate the main process.
            # Using a QTimer to ensure the event loop has time to clean up before killing.
            from PyQt6.QtCore import QTimer

            QTimer.singleShot(100, terminate)
        else:
            event.ignore()


def terminate():
    """Terminate the application"""
    os._exit(0)


def run_gui(args):
    """Run the GUI application in a separate thread"""
    global app_instance
    app = QApplication(sys.argv)
    app_instance = SnapViewerApp(args)
    app_instance.show()
    app.exec()  # Start the Qt event loop

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
