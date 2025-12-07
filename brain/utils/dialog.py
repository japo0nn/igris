from typing import List, Dict
from llm_client import LLMClient


class Dialog:
    def __init__(self, provider: str = "offline"):
        self.client = LLMClient(provider)
        self.history: List[Dict[str, str]] = []

    def send(self, user_message: str) -> str:
        # добавляем сообщение пользователя в историю
        self.history.append({"role": "user", "content": user_message})

        # формируем prompt
        response = self.client.ask(user_message)

        # сохраняем ответ ассистента
        self.history.append({"role": "assistant", "content": response})
        return response

    def get_history(self) -> List[Dict[str, str]]:
        return self.history
