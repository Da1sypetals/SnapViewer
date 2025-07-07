#!/usr/bin/env python3
"""
SnapViewer GUI using PyQt6
Features:
- Left panel: Displays messages from main thread
- Right panel: Echo REPL
- Main thread runs viewer() function
- Process communication via multiprocessing.Queue and QTimer
"""

import argparse
import os
import sys
import multiprocessing
import time  # Added for waiting/polling
from datetime import datetime
from PyQt6.QtCore import Qt, QTimer
from PyQt6.QtGui import QFont, QShortcut, QKeySequence, QFontDatabase
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
# This will now be set within the viewer_process, not the main orchestrator process.
snapviewer = None


class MessagePanel(QWidget):
    """Panel that displays messages from main thread"""

    def __init__(self, parent=None):
        super().__init__(parent)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(15)
        # Configure fonts
        self.title_font = QFont("JetBrains Mono", 20, QFont.Weight.Bold)
        self.mono_font = QFont("JetBrains Mono", 14)
        # Title
        title_label = QLabel("Messages")
        title_label.setFont(self.title_font)
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        title_label.setStyleSheet("""
            QLabel {
                color: #e91e63;
                padding: 10px;
                border-bottom: 2px solid #e91e63;
                margin-bottom: 10px;
            }
        """)
        layout.addWidget(title_label)
        # Message display
        self.text_widget = QTextEdit()
        self.text_widget.setFont(self.mono_font)
        self.text_widget.setReadOnly(True)
        self.text_widget.setStyleSheet("""
            QTextEdit {
                background-color: #fce4ec;
                border: 2px solid #f3d4dc;
                border-radius: 10px;
                padding: 15px;
                color: #2d2d2d;
                selection-background-color: #e91e63;
                selection-color: white;
            }
            QTextEdit:focus {
                border: 2px solid #e91e63;
                background-color: #f3d4dc;
            }
            QScrollBar:vertical {
                background: #f3d4dc;
                width: 12px;
                border-radius: 6px;
            }
            QScrollBar::handle:vertical {
                background: #e91e63;
                border-radius: 6px;
                min-height: 20px;
            }
            QScrollBar::handle:vertical:hover {
                background: #c2185b;
            }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
                height: 0px;
            }
        """)
        layout.addWidget(self.text_widget)
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
        "SQLite REPL - This is a SQLite database storing the allocation data.",
        "Type `--help` to see available commands",
        "Ctrl+D to quit application",
        "Ready for your queries!",
    ]

    def __init__(self, args, parent=None):
        super().__init__(parent)
        self.args = args
        layout = QVBoxLayout(self)
        layout.setContentsMargins(20, 20, 20, 20)
        layout.setSpacing(15)
        # Configure fonts
        self.title_font = QFont("JetBrains Mono", 20, QFont.Weight.Bold)
        self.mono_font = QFont("JetBrains Mono", 14)
        # Title
        title_label = QLabel("SQLite REPL")
        title_label.setFont(self.title_font)
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        title_label.setStyleSheet("""
            QLabel {
                color: #e91e63;
                padding: 10px;
                border-bottom: 2px solid #e91e63;
                margin-bottom: 10px;
            }
        """)
        layout.addWidget(title_label)
        # Output area
        self.output_text = QTextEdit()
        self.output_text.setFont(self.mono_font)
        self.output_text.setReadOnly(True)
        self.output_text.setStyleSheet("""
            QTextEdit {
                background-color: #fce4ec;
                border: 2px solid #f3d4dc;
                border-radius: 10px;
                padding: 15px;
                color: #2d2d2d;
                selection-background-color: #e91e63;
                selection-color: white;
            }
            QTextEdit:focus {
                border: 2px solid #e91e63;
                background-color: #f3d4dc;
            }
            QScrollBar:vertical {
                background: #f3d4dc;
                width: 12px;
                border-radius: 6px;
            }
            QScrollBar::handle:vertical {
                background: #e91e63;
                border-radius: 6px;
                min-height: 20px;
            }
            QScrollBar::handle:vertical:hover {
                background: #c2185b;
            }
            QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
                height: 0px;
            }
        """)
        layout.addWidget(self.output_text)
        # Input area
        input_frame = QWidget()
        input_frame.setStyleSheet("""
            QWidget {
                background-color: #f3d4dc;
                border-radius: 10px;
                padding: 5px;
            }
        """)
        input_layout = QHBoxLayout(input_frame)
        input_layout.setContentsMargins(15, 10, 15, 10)
        prompt_label = QLabel("> ")
        prompt_label.setStyleSheet("""
            QLabel {
                color: #e91e63;
                font-weight: bold;
                font-size: 16px;
                background: transparent;
                border: none;
                padding: 0px;
            }
        """)
        input_layout.addWidget(prompt_label)
        self.input_entry = HistoryLineEdit()
        self.input_entry.setFont(self.mono_font)
        self.input_entry.setPlaceholderText("Enter SQL command or --help for options...")
        self.input_entry.returnPressed.connect(self.on_submit)
        self.input_entry.setStyleSheet("""
            QLineEdit {
                background-color: white;
                border: 1px solid #e91e63;
                border-radius: 5px;
                padding: 8px;
                color: #2d2d2d;
                selection-background-color: #e91e63;
                selection-color: white;
            }
            QLineEdit:focus {
                border: 2px solid #e91e63;
                background-color: #fff5f8;
            }
        """)
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
                    # Access the global snapviewer instance here
                    if snapviewer:
                        output = snapviewer.execute_sql(command)
                        self.output_lines.append(f"[{timestamp}]\n{output}")
                    else:
                        self.output_lines.append(f"[{timestamp}]\nError: Viewer not initialized.")
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
    """Main GUI application with process communication"""

    def __init__(self, args, message_queue, terminate_event):
        super().__init__()
        self.args = args
        self.message_queue = message_queue
        self.terminate_event = terminate_event  # Add the terminate event
        self.setWindowTitle("SnapViewer - Memory Allocation Viewer & SQLite REPL")
        self.setGeometry(100, 100, 1600, 1200)
        # Set application-wide style
        self.setStyleSheet("""
            QMainWindow {
                background: qlineargradient(x1: 0, y1: 0, x2: 0, y2: 1,
                                            stop: 0 #fff5f8, stop: 1 #fce4ec);
            }
            QWidget {
                font-family: 'JetBrains Mono', monospace;
            }
            QMessageBox {
                background-color: #fce4ec;
                color: #2d2d2d;
                font-family: 'JetBrains Mono', monospace;
            }
            QMessageBox QPushButton {
                background-color: #e91e63;
                color: white;
                border: none;
                padding: 8px 16px;
                border-radius: 5px;
                font-weight: bold;
                font-family: 'JetBrains Mono', monospace;
            }
            QMessageBox QPushButton:hover {
                background-color: #c2185b;
            }
            QMessageBox QPushButton:pressed {
                background-color: #ad1457;
            }
        """)
        # Setup timer to check the message queue and termination event
        self.timer = QTimer(self)
        self.timer.timeout.connect(self.check_message_queue)
        self.timer.timeout.connect(self.check_termination_event)  # Check termination event
        self.timer.start(100)  # Check every 100ms
        # Create main container
        main_widget = QWidget()
        main_widget.setStyleSheet("""
            QWidget {
                background: transparent;
            }
        """)
        self.setCentralWidget(main_widget)
        main_layout = QHBoxLayout(main_widget)
        main_layout.setContentsMargins(20, 20, 20, 20)
        main_layout.setSpacing(20)
        # Create panels
        self.message_panel = MessagePanel()
        self.repl_panel = REPLPanel(args)
        # Style the panels
        panel_style = """
            QWidget {
                background-color: rgba(248, 187, 217, 0.3);
                border-radius: 15px;
                border: 1px solid #f3d4dc;
            }
        """
        self.message_panel.setStyleSheet(panel_style)
        self.repl_panel.setStyleSheet(panel_style)
        main_layout.addWidget(self.message_panel, 1)
        # Add separator
        separator = QFrame()
        separator.setFrameShape(QFrame.Shape.VLine)
        separator.setFrameShadow(QFrame.Shadow.Sunken)
        separator.setStyleSheet("""
            QFrame {
                color: #e91e63;
                background-color: #e91e63;
                border: none;
                max-width: 2px;
            }
        """)
        main_layout.addWidget(separator)
        main_layout.addWidget(self.repl_panel, 1)
        # Add keyboard shortcuts
        QShortcut(QKeySequence("Ctrl+D"), self, self.close)
        QShortcut(QKeySequence("Ctrl+Q"), self, self.close)

    def check_message_queue(self):
        """Check for messages from the main process"""
        while not self.message_queue.empty():
            message = self.message_queue.get_nowait()
            self.update_message(message)

    def check_termination_event(self):
        """Check if the main process has signaled termination"""
        if self.terminate_event.is_set():
            print("GUI: Termination signal received from another process. Quitting.")
            # Quit the QApplication cleanly
            QApplication.instance().quit()

    def update_message(self, message: str):
        """Update the message panel content"""
        self.message_panel.update_content(message)

    def closeEvent(self, event):
        """Handle window close event"""
        # Create a custom styled message box
        msg_box = QMessageBox(self)
        msg_box.setWindowTitle("Quit SnapViewer")
        msg_box.setText("Are you sure you want to quit SnapViewer?")
        msg_box.setIcon(QMessageBox.Icon.Question)
        msg_box.setStandardButtons(QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No)
        msg_box.setDefaultButton(QMessageBox.StandardButton.No)
        reply = msg_box.exec()
        if reply == QMessageBox.StandardButton.Yes:
            event.accept()
            # Set the termination event to signal other processes to exit
            self.terminate_event.set()
            # Allow event loop to process the quit signal
            QTimer.singleShot(0, QApplication.instance().quit)
        else:
            event.ignore()


def run_gui(args, message_queue, terminate_event):
    """Run the GUI application in a separate process"""
    app = QApplication(sys.argv)
    # Load the JetBrains Mono font
    font_path = os.path.join(os.path.dirname(__file__), "assets", "JetBrainsMono-Medium.ttf")
    if os.path.exists(font_path):
        font_id = QFontDatabase.addApplicationFont(font_path)
        if font_id == -1:
            print(f"Warning: Could not load font from {font_path}")
    else:
        print(f"Warning: Font file not found at {font_path}")
    # Set application icon and other properties
    app.setApplicationName("SnapViewer")
    app.setApplicationVersion("1.0")
    app.setApplicationDisplayName("SnapViewer - Memory Allocation Viewer")
    app_instance = SnapViewerApp(args, message_queue, terminate_event)
    app_instance.show()
    app.exec()  # Start the Qt event loop
    print("GUI process stopped.")


def run_viewer(args, message_queue, terminate_event):
    """Run the SnapViewer logic in a separate process"""
    global snapviewer
    snapviewer = SnapViewer(args.path, args.resolution, args.log)

    def message_callback(message: str):
        """Callback that sends messages to the GUI process via a queue"""
        # Only put messages if the terminate event is not set, to avoid putting into a closed queue
        if not terminate_event.is_set():
            message_queue.put(message)

    try:
        print("Viewer process started.")
        # This is the blocking call into Rust extension.
        # It needs to periodically check the terminate_event or be designed
        # to break its loop if a signal is received.
        # For now, we assume snapviewer.viewer is an infinite loop that can be
        # interrupted by a KeyboardInterrupt.
        # We also need a way for the terminate_event to stop the viewer gracefully.
        # This typically means the viewer itself should check the event, or we use
        # a more aggressive termination.
        snapviewer.viewer(message_callback)  # If viewer itself does not check for event, it will block.
    except KeyboardInterrupt:
        print("\nViewer: KeyboardInterrupt detected. Signalling termination.")
    except Exception as e:
        print(f"Viewer: Error in viewer: {e}. Signalling termination.")
    finally:
        # Signal all other processes to terminate
        terminate_event.set()
        print("Viewer process signalling termination.")


def main():
    """Run the application orchestrator"""
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
        sys.exit(1)  # Use sys.exit for clean exit in main thread

    # Create communication channels
    message_queue = multiprocessing.Queue()
    terminate_event = multiprocessing.Event()

    # Start GUI in a separate process
    gui_process = multiprocessing.Process(
        target=run_gui, args=(args, message_queue, terminate_event), daemon=True
    )
    gui_process.start()

    # Start Viewer in a separate process
    viewer_process = multiprocessing.Process(
        target=run_viewer, args=(args, message_queue, terminate_event), daemon=True
    )
    viewer_process.start()

    print("Main orchestrator process started. Waiting for termination signal...")

    # Main orchestrator loop: wait for either process to signal termination
    try:
        while not terminate_event.is_set():
            # Check if either process has exited unexpectedly
            if not gui_process.is_alive():
                print("Main: GUI process died unexpectedly. Signalling termination.")
                terminate_event.set()
                break
            if not viewer_process.is_alive():
                print("Main: Viewer process died unexpectedly. Signalling termination.")
                terminate_event.set()
                break
            time.sleep(0.1)  # Poll for the event

    except KeyboardInterrupt:
        print("\nMain: KeyboardInterrupt detected. Signalling termination to all processes.")
        terminate_event.set()  # Set the event to signal termination

    finally:
        print("Main: Waiting for processes to join...")
        # If processes are still alive, forcefully terminate them
        if gui_process.is_alive():
            print("Main: GUI process did not terminate gracefully, forcing exit.")
            gui_process.terminate()
        if viewer_process.is_alive():
            print("Main: Viewer process did not terminate gracefully, forcing exit.")
            viewer_process.terminate()

        print("Main: All processes shut down. Exiting application.")
        sys.exit(0)  # Use sys.exit for a clean program exit


if __name__ == "__main__":
    # Ensure multiprocessing starts cleanly on all platforms
    multiprocessing.freeze_support()
    main()
