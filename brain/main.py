import asyncio

import grpc
from core.message import BrainService
from generated import brain_pb2_grpc
from concurrent import futures
from utils.dialog import Dialog

def serve():
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    brain_pb2_grpc.add_BrainRouterServicer_to_server(BrainService(), server)
    server.add_insecure_port("[::]:50051")
    server.start()
    print("Server started on :50051")
    server.wait_for_termination()

if __name__ == "__main__":
    dialog = Dialog(provider="offline")  # или "openai"
    
    serve()

