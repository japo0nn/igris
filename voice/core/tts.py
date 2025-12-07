import pyttsx3
import queue
import threading

tts_queue = queue.Queue()
engine = pyttsx3.init(driverName='sapi5')
engine.setProperty('rate', 150)
engine.setProperty('volume', 1.0)

def tts_loop():
    while True:
        text = tts_queue.get()
        if text is None:
            break
        engine.say(text)
        engine.runAndWait()
        tts_queue.task_done()

threading.Thread(target=tts_loop, daemon=True).start()

def speak(text: str):
    print(f"[TTS] Speaking: {text}")
    tts_queue.put(text)
