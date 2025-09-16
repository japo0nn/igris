import gettext
import os
import sys
import threading
from mpv import MPV

class Player:
    def __init__(self, mpv_instance: MPV):
        self.player = mpv_instance
        self.stop_event = threading.Event()
    
    def play(self, url: str):
        self.stop_event.clear()
        self.player.play(url)
        self.player.wait_until_playing()

    def pause(self):
        self.player.pause = True
        return self.player.pause
    
    def resume(self):
        self.player.pause = False
        return self.player.pause

    def stop(self):
        self.player.stop()
        self.stop_event.set()

    def is_stopped(self):
        return self.stop_event.is_set()

    def wait_for_end(self, callback=None):
        while not self.stop_event.is_set():
            event = self.player.wait_for_event(0.1)
            if event and event.get("event_id") == "end-file":
                if callback:
                    callback()
                break