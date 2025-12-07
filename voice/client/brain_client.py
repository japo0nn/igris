import grpc

from generated import brain_pb2, brain_pb2_grpc
from core.tts import speak


def call_brain(message):
    channel = grpc.insecure_channel("localhost:50051")
    stub = brain_pb2_grpc.BrainRouterStub(channel)

    msg = brain_pb2.IgrisMessage(
        type=brain_pb2.EVENT,
        source=brain_pb2.VOICE,
        target=brain_pb2.BRAIN,
        text=brain_pb2.TextPayload(text=message)
    )

    response = stub.GetResponse(msg)
    if response.text:
        speak(response.text.text)