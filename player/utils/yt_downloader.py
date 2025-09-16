import yt_dlp

def get_audio_url(video_id):
    url = f"https://www.youtube.com/watch?v={video_id}"
    opts = {
        'format': 'bestaudio/best',
        'quiet': True,
        'noplaylist': True,
        'skip_download': True,
    }

    with yt_dlp.YoutubeDL(opts) as ydl:
        info = ydl.extract_info(url, download=False)
        return info['url']