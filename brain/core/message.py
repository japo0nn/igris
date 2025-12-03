from brain.generated import brain_pb2_grpc
from brain.generated import brain_pb2
from utils.dialog import Dialog

class BrainService(brain_pb2_grpc.BrainRouterServicer):

    def __init__(self):
        self.dialog = Dialog(provider="offline")
        pass

    def GetResponse(self, request, context):
        type = request.type

        if (request.type == brain_pb2.EVENT):
            answer = self.dialog.send(request.text.text)
        elif (request.type == brain_pb2.COMMAND):
            print("This is command")
        elif (request.type == brain_pb2.RESPONSE):
            print("This is response")