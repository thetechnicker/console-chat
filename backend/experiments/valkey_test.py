import valkey

r = valkey.Valkey(host="localhost", port=6379, protocol=3)
print(r.ping())
