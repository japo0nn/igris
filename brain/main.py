import asyncio
from utils.dialog import Dialog


# async def main():

#     dialog = Dialog()
#     core = CoreClient(dialog)

#     await core.connect()


if __name__ == "__main__":
    dialog = Dialog(provider="offline")  # или "openai"
    
    while True:
        user_inp = input("You: ")
        if user_inp.lower() in {"exit", "quit"}:
            break
        answer = dialog.send(user_inp)
        print("AI:", answer)

