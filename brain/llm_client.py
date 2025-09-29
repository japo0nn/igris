import os
import requests
from openai import OpenAI

class LLMClient:
    def __init__(self, mode="offline"):
        if mode == "online" and os.getenv("OPENAI_API_KEY") != None:
            self.client = OpenAIClient()
        elif mode == "offline":
            self.client = OllamaClient()
        else:
            raise ValueError("Unknown provider")

    def ask(self, prompt: str) -> str:
        return self.client.ask(prompt)
    

class OpenAIClient:
    def __init__(self):
        self.client = OpenAI(api_key=os.getenv("OPENAI_API_KEY"))

    def ask(self, prompt: str) -> str:
        resp = self.client.chat.completions.create(
            model="gpt-4o-mini",
            messages=[{"role": "user", "content": prompt}]
        )
        return resp.choices[0].message.content

    

class OllamaClient:
    def __init__(self, model="llama3"):
        self.url = "http://localhost:11434/api/generate"
        self.model = model

    def ask(self, prompt: str) -> str:
        resp = requests.post(self.url, json={"model": self.model, "prompt": prompt})
        return resp.json()["response"]

