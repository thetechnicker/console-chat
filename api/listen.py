import asyncio

import httpx

API_BASE_URL = "http://localhost:8000"


async def get_anonymous_token():
    url = f"{API_BASE_URL}/login"
    async with httpx.AsyncClient() as client:
        response = await client.post(url, json={})  # Empty JSON body
        response.raise_for_status()
        data = response.json()
        return data.get("token")


async def listen_room_abc(token: str, listen_seconds: int = 30):
    url = f"{API_BASE_URL}/room/abc"
    headers = {"Authorization": f"Bearer {token}"}
    params = {"listen_seconds": listen_seconds}
    async with httpx.AsyncClient(timeout=1000) as client:
        while True:
            try:
                async with client.stream(
                    "GET", url, headers=headers, params=params
                ) as response:
                    async for chunk in response.aiter_text():
                        print(chunk)
                        # if "END" in chunk:
                        #    break
            except httpx.HTTPStatusError as e:
                print(f"HTTP error: {e.response.status_code}, reconnecting...")
            except Exception as e:
                print(f"Error: {e}, reconnecting...")
            await asyncio.sleep(1)


async def main():
    token = await get_anonymous_token()
    print("Obtained token:", token)
    if token:
        try:
            await listen_room_abc(token)
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    asyncio.run(main())
