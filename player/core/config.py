import os

MPV_DIR = os.path.join(os.path.dirname(__file__), "..", "mpv")
os.environ["PATH"] = MPV_DIR + os.pathsep + os.environ.get("PATH", "")
