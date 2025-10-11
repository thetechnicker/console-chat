import httpx
import asyncio
import time

TOKEN_REFRESH_MARGIN = 10  # seconds before expiry to refresh token


async def refresh_token_periodically(token_info):
    async with httpx.AsyncClient() as client:
        while True:
            expires_in = token_info["expires_at"] - time.time()
            # Sleep until just before token expiry
            if expires_in > TOKEN_REFRESH_MARGIN:
                await asyncio.sleep(expires_in - TOKEN_REFRESH_MARGIN)
            # Refresh the token
            response = await client.get("http://localhost:8000/api/status")
            response.raise_for_status()
            data = response.json()
            token_info["token"] = data["token"]
            token_info["expires_at"] = time.time() + data["ttl"]
            print(f"Token refreshed, expires in {data['ttl']} seconds.")


async def listen_room(token_info, room):
    url = f"http://localhost:8000/api/r/{room}"
    async with httpx.AsyncClient(timeout=1000) as client:
        headers = {}
        while True:
            headers["Authorization"] = f"Bearer {token_info['token']}"
            try:
                async with client.stream("GET", url, headers=headers) as response:
                    async for chunk in response.aiter_text():
                        print(chunk)
            except httpx.HTTPStatusError as e:
                print(f"HTTP error: {e.response.status_code}, reconnecting...")
            except Exception as e:
                print(f"Error: {e}, reconnecting...")
            await asyncio.sleep(1)


async def main():
    async with httpx.AsyncClient() as client:
        # Fetch initial token and TTL
        response = await client.get("http://localhost:8000/api/status")
        response.raise_for_status()
        data = response.json()
        token_info = {"token": data["token"], "expires_at": time.time() + data["ttl"]}

    # Start concurrent tasks for listening and refreshing token
    listen_task = asyncio.create_task(listen_room(token_info, "abc"))
    refresh_task = asyncio.create_task(refresh_token_periodically(token_info))

    # Run both tasks concurrently
    await asyncio.gather(listen_task, refresh_task)


asyncio.run(main())
