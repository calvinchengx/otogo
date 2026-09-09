# Request: store a secret and read it back with the real Python SDK

A developer points the unmodified `azure-keyvault-secrets` client at the
emulator, using `DefaultAzureCredential`, and does the most ordinary thing
there is.

```python
client.set_secret("db-password", "hunter2")
client.get_secret("db-password")
```

## What the user expects

- The credential walks the Entra challenge without any emulator-specific code.
- `set_secret` returns the secret object with an id, a version, and enabled=true.
- `get_secret` returns **the same value**, from a separate call.

## Persistent effect to confirm

After the client process has exited, a **fresh** client must still read
`hunter2` back. Confirm out-of-band, not from the first response body.
