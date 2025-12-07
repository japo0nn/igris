import json
import queue
import sounddevice as sd
import sys
import numpy as np

import vosk

q = queue.Queue()

def callback(indata, frames, time, status):
    if status:
        print(status, file=sys.stderr)

    data = np.frombuffer(indata, dtype=np.int16)

    # downmix если 2 канала
    if len(data) % 2 == 0 and data.size >= 2:
        data = data.reshape(-1, 2)
        mono = data.mean(axis=1).astype(np.int16)
        q.put(mono.tobytes())
    else:
        q.put(data.tobytes())

def recognize(model_path="vosk-model", device=None):
    model = vosk.Model(model_path)
    rec = vosk.KaldiRecognizer(model, 16000)

    device_info = sd.query_devices(device)
    samplerate = int(device_info['default_samplerate'])
    channels = device_info['max_input_channels']

    with sd.RawInputStream(
        samplerate=samplerate,
        blocksize=8000,
        dtype='int16',
        channels=channels,          # <--- важно
        callback=callback,
        device=device
):
        print("Listening...")
        while True:
            data = q.get()  # bytes

            data_int16 = np.frombuffer(data, dtype=np.int16)

            if samplerate != 16000:
                resampled = np.interp(
                    np.linspace(0, len(data_int16), int(len(data_int16) * 16000 / samplerate)),
                    np.arange(len(data_int16)),
                    data_int16
                ).astype(np.int16)
                data16k = resampled.tobytes()
            else:
                data16k = data

            if rec.AcceptWaveform(data16k):
                res = json.loads(rec.Result())
                text = res.get("text", "")
                if text:
                    print("You:", text)
