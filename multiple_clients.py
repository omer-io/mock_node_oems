import threading
import time
import websocket

SERVER_URL = "ws://127.0.0.1:3000"  # your echo server
NUM_CLIENTS = 10     # configurable
NUM_MESSAGES = 100   # per client


def run_client(client_id: int, server_url: str, num_messages: int):
    ws = websocket.WebSocket()
    ws.connect(server_url)
    print(f"Client {client_id} connected")

    total_latency = 0.0

    for i in range(num_messages):
        msg = f"Hello from client {client_id} msg {i}"
        start = time.perf_counter()

        ws.send(msg)
        reply = ws.recv()

        latency = (time.perf_counter() - start) * 1000  # ms
        total_latency += latency

        # print(f"Client {client_id} got reply: {reply}, latency: {latency:.2f} ms")

    avg_latency = total_latency / num_messages
    print(f"Client {client_id} done. Avg latency: {avg_latency:.2f} ms")

    ws.close()


def main():
    threads = []

    for client_id in range(NUM_CLIENTS):
        t = threading.Thread(
            target=run_client,
            args=(client_id, SERVER_URL, NUM_MESSAGES),
            daemon=True
        )
        threads.append(t)
        t.start()

    for t in threads:
        t.join()


if __name__ == "__main__":
    main()
