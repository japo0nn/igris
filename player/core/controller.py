import gettext
import os
import sys
import core.config as config
from utils.yt_downloader import get_audio_url
from utils.ytmusic_api import search_songs
from core.player import Player
from mpv import MPV

class Controller:
    def __init__(self):
        self.player = Player(MPV(ytdl=True))
        self.queue = []
        self.current_index = -1

    def add_to_queue(self, video_id: str):
        self.queue.append(video_id)

    def play_index(self, index: int):
        if index < 0 or index >= len(self.queue):
            return
        self.current_index = index
        url = get_audio_url(self.queue[index])
        self.player.play(url)

    def next(self):
        if self.current_index + 1 < len(self.queue):
            self.play_index(self.current_index + 1)

    def prev(self):
        if self.current_index - 1 >= 0:
            self.play_index(self.current_index - 1)

    def pause(self):
        return self.player.pause()
    
    def resume(self):
        return self.player.resume()

    def stop(self):
        self.player.stop()

    def search_and_play(self, query: str):
        results = search_songs(query)
        if results:
            self.queue = [item['videoId'] for item in results]
            self.play_index(0)
