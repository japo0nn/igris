import sounddevice as sd

from core.stt import recognize

def get_input_device():
    for i, dev in enumerate(sd.query_devices()):
        if dev['max_input_channels'] > 0:
            return i
    raise RuntimeError("No input device found")


if __name__ == "__main__":
    try:
        device_index = get_input_device()
        device_info = sd.query_devices(device_index)
        samplerate = int(device_info['default_samplerate'])
        channels = device_info['max_input_channels']

        print(f"Using device #{device_index}: {device_info['name']} with {channels} channels at {samplerate} Hz")

        recognize(r"C:/Users/sosa/Documents/Workspace/igris/voice/core/vosk-model", device_index)
    except KeyboardInterrupt:
        print("\nStopped")