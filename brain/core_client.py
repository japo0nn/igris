import grpc
from igris_pb2 import IgrisMessage
from generated.igris_pb2_grpc import CoreRouterStub


class CoreClient:
    def __init__(self, dialog):
        self.dialog = dialog
        self.channel = grpc.aio.insecure_channel("localhost:50051")
        self.stub = CoreRouterStub(self.channel)

    async def connect(self):
        async def message_generator():
            while True:
                # ждём события от dialog, чтобы отправить в Core
                msg = await self.dialog.get_outgoing_message()
                yield IgrisMessage(
                    type=msg["type"],
                    source=msg["source"],
                    target=msg["target"],
                    payload=msg["payload"]
                )

        async for incoming in self.stub.Connect(message_generator()):
            # передаем пришедший запрос в dialog (OpenAI)
            await self.dialog.handle_incoming({
                "type": incoming.type,
                "source": incoming.source,
                "target": incoming.target,
                "payload": incoming.payload
            })
