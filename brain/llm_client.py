import os
import json
import requests
from openai import OpenAI
from ollama import chat
from ollama import ChatResponse


class LLMClient:
    def __init__(self, mode="offline"):
        with open("system_prompt.txt", "r", encoding="utf-8") as f:
            self.system_prompt = f.read()

        if mode == "online" and os.getenv("OPENAI_API_KEY") != None:
            self.client = OpenAIClient()
        elif mode == "offline":
            self.client = OllamaClient()
        else:
            raise ValueError("Unknown provider")

    def load_system_prompt():
        with open("system_prompt.txt", "r", encoding="utf-8") as f:
            return f.read()
        
    def ask(self, prompt: str) -> str:
        return self.client.ask(prompt, self.system_prompt)
    
    

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
    def __init__(self, model="gemma3:4b"):
        self.url = "http://localhost:11434/api/generate"
        self.model = model

    def ask(self, prompt: str, system_prompt: str) -> str:
        response: ChatResponse = chat(model='gemma3:4b', messages=[
            {
                'role': 'system',
                'content': system_prompt,
            },
            {
                'role': 'user',
                'content': prompt,
            },
        ])
        # or access fields directly from the response object
        return response.message.content

