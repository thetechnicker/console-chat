import httpx


async def listen_stream():
    url = "http://localhost:8000/api/r/abc"
    async with httpx.AsyncClient(timeout=1000) as client:
        async with client.stream("GET", url) as response:
            async for chunk in response.aiter_text():
                print(chunk)


import asyncio

asyncio.run(listen_stream())
