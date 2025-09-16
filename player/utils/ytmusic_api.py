import gettext
import os
import sys

from ytmusicapi import YTMusic


ytmusic = YTMusic()
    
def search_songs(query: str):
    return ytmusic.search(query, filter="songs", limit=10)

def get_related_songs(video_id: str):
    return ytmusic.get_song_related()