import json
from generated import brain_pb2_grpc
from generated import brain_pb2
from utils.dialog import Dialog

class BrainService(brain_pb2_grpc.BrainRouterServicer):

    def __init__(self):
        self.dialog = Dialog(provider="offline")
        pass

    def GetResponse(self, request, context):
        type = request.type

        if (request.type == brain_pb2.EVENT):
            answer = self.dialog.send(request.text.text)
            # print()
            res = parse_ollama_to_grpc(answer)
            print(res)
            return res
        elif (request.type == brain_pb2.COMMAND):
            print("This is command")
        elif (request.type == brain_pb2.RESPONSE):
            print("This is response")


def parse_ollama_to_grpc(json_str):
    # убираем ``` если есть
    clean_str = json_str.strip().strip("`").strip("json")
    print (clean_str)
    data = json.loads(clean_str)

    print(data)
    
    msg_kwargs = {
        "type": data["type"],
        "source": data["source"],
        "target": data["target"]
    }
    
    # payload
    if "text" in data:
        msg_kwargs["text"] = brain_pb2.TextPayload(text=data["text"]["text"])
    elif "command" in data:
        msg_kwargs["command"] = brain_pb2.CommandPayload(
            command=data["command"]["command"],
            args=data["command"].get("args", [])
        )
    elif "result" in data:
        msg_kwargs["result"] = brain_pb2.ResultPayload(
            message=data["result"]["message"],
            success=data["result"]["success"]
        )
    
    return brain_pb2.IgrisMessage(**msg_kwargs)