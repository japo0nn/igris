import gettext
import locale
import os
import socket
import sys
import threading
from core.controller import Controller

locale.setlocale(locale.LC_NUMERIC, "C")

HOST = "127.0.0.1"
PORT = 8765

controller = Controller()

def handle_client(conn):
    with conn:
        data = conn.recv(1024).decode().strip()
        if not data:
            return
        parts = data.split(" ", 1)
        cmd = parts[0].lower()
        arg = parts[1] if len(parts) > 1 else None

        if cmd == "play" and arg:
            controller.search_and_play(arg)
        elif cmd == "pause":
            controller.pause()
        elif cmd == "resume":
            controller.resume()
        elif cmd == "stop":
            controller.stop()
        elif cmd == "next":
            controller.next()
        elif cmd == "prev":
            controller.prev()

def server_loop():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind((HOST, PORT))
        s.listen()
        print("Daemon running…")
        while True:
            conn, addr = s.accept()
            threading.Thread(target=handle_client, args=(conn,), daemon=True).start()

if __name__ == "__main__":
    server_loop()
