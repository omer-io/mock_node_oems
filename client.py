# import asyncio
# import websockets

# async def hello():
#     uri = "ws://localhost:3000"
#     async with websockets.connect(uri) as websocket:
#         message = "Hello, WebSocket!"
#         print(f"Sending: {message}")
#         await websocket.send(message)

#         response = await websocket.recv()
#         print(f"Received: {response}")

# asyncio.run(hello())


# import asyncio
# import websockets

# async def interactive_client():
#     uri = "ws://localhost:3000"
#     async with websockets.connect(uri) as websocket:
#         print("Connected to WebSocket server.")
#         while True:
#             message = input("Enter message to send (or 'exit' to quit): ")
#             if message.lower() == 'exit':
#                 break
#             await websocket.send(message)
#             response = await websocket.recv()
#             print(f"Received: {response}")

# asyncio.run(interactive_client())


import asyncio
import websockets

async def auto_client():
    uri = "ws://localhost:3000"
    async with websockets.connect(uri) as websocket:
        print("Connected. Sending messages every 2 seconds...")
        counter = 0
        while True:
            msg = f"Message {counter}"
            await websocket.send(msg)
            print(f"Sent: {msg}")
            reply = await websocket.recv()
            print(f"Received: {reply}")
            counter += 1
            # await asyncio.sleep(2)

asyncio.run(auto_client())
