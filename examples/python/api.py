"""Some API client code that needs cleanup."""
import json
import os
import sys
from urllib.request import urlopen, Request

API_KEY = os.environ.get("API_KEY", "dev-key")


def fetch_user(user_id):
    url = f"https://api.example.com/users/{user_id}"
    request = Request(url, headers={"Authorization": f"Bearer {API_KEY}"})
    response = urlopen(request)
    data = json.loads(response.read())
    print("fetch_user returned: " + str(data))
    return data


def create_user(name, email):
    url = "https://api.example.com/users"
    body = json.dumps({"name": name, "email": email}).encode()
    request = Request(url, data=body, headers={
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json",
    })
    response = urlopen(request)
    result = json.loads(response.read())
    print("create_user returned: " + str(result))
    return result


def delete_user(user_id):
    url = f"https://api.example.com/users/{user_id}"
    request = Request(url, method="DELETE", headers={
        "Authorization": f"Bearer {API_KEY}",
    })
    response = urlopen(request)
    print("delete completed for: " + str(user_id))
