<div align="center">

# ChattyNano

### Back to [Maxwell](https://developer.nvidia.com/maxwell-compute-architecture)

<img src="assets/jetson-waveshare-kit.jpg" width="1152" height="1024">

</div>

Bringing **on-device** Speech Recognition (STT) and Text Generation (LLM) pipeline to an old [NVIDIA Jetson Nano](https://developer.nvidia.com/embedded/learn/get-started-jetson-nano-devkit) from 2019 with $${\color{purple}vibes}$$.

This might be the perfect project if you somehow forgot about owning a Jetson Nano board and want to make it busy doing stuff. **For fun only**—the board is old, so most of the technologies used are **deprecated**, **outdated**, or **superseded** in one way or another, but it's still insanely cool.

**This project was heavily inspired by [whisper-edge](https://github.com/maxbbraun/whisper-edge)**

Verified on:
- Jetson Nano 4GB, rev A02
- Waveshare Audio Card for Jetson Nano
- [L4T 32.7.1](https://developer.nvidia.com/embedded/linux-tegra-r3271), [JetPack 4.6.1](https://developer.nvidia.com/embedded/jetpack-sdk-461)

# Fire that thang ✨

Your typical use-case is the following:
1. You ask stuff with the mic
2. STT transcribes stuff and passes the transcription to LLM
3. LLM responds

```bash
docker build -f Dockerfile -t chatty_nano:dev .
# for locally running LLM server
sudo bash run.sh --model-dir "path/to/coqui/tts/models" --openai-endpoint "http://localhost:8000/v1" --openai-model "tinyllama-1.1b-chat" --openai-api-key "123"
# or:
sudo bash run.sh --help
```

```console
<...>
Using input device: plughw:CARD=Device,DEV=0
Default input config: SupportedStreamConfig { channels: 2, sample_rate: SampleRate(44100), buffer_size: Range { min: 2, max: 4294967294 }, sample_format: F32 }
Recording complete. Transcribing...

Transcription:
hello there what is the meaning of life

Transcription time: 3.492165433s

Sending transcription to AI server...

AI Response:
The meaning of life is an enduring and complex question that has fascinated humans for centuries. It can be interpreted as a philosophical, spiritual or religious inquiry that seeks to find deeper meanings in human existence. There are no definitive answers, but there are many perspectives and opinions on what the meaning of life may entail.
<...>
```

The following arguments are accepted for configuration:

```bash
Speech-to-Text application using Coqui STT

Usage: jetson-stt [OPTIONS] --model-dir <MODEL_DIR>

Options:
  -m, --model-dir <MODEL_DIR>
          Model directory containing the model files
      --chunk-seconds <CHUNK_SECONDS>
          The length in seconds to wait for audio recording to transcribe [default: 10]
      --input-device <INPUT_DEVICE>
          The input device used to record audio [default: plughw:CARD=Device,DEV=0]
      --num-channels <NUM_CHANNELS>
          The number of channels of the recorded audio [default: 1]
      --sample-rate <SAMPLE_RATE>
          The sample rate of the recorded audio [default: 16000]
      --openai-endpoint <OPENAI_ENDPOINT>
          OpenAI API endpoint (default: https://api.openai.com/v1) [default: https://api.openai.com/v1]
      --openai-model <OPENAI_MODEL>
          OpenAI model name (e.g., gpt-4, gpt-3.5-turbo) [default: gpt-3.5-turbo]
      --openai-api-key <OPENAI_API_KEY>
          OpenAI API key (optional, can be set via OPENAI_API_KEY env var)
  -h, --help
          Print help
  -V, --version
          Print version
```

## STT
Coqui STT running on CPU is used.
_You can consider this a tribute to the great work and reflect a little on how far things have come in just 6 years._
  
Download the STT models you're interested in from [here](https://github.com/coqui-ai/STT-models/releases), `*.tflite` and `*.scorer`, put them in `data/models`

The credit goes to:
- [**`coqui-ai/STT`**](https://github.com/coqui-ai/STT)
- [**`tazz4843/coqui-stt`**](https://github.com/tazz4843/coqui-stt)
- [**`domcross/DeepSpeech-for-Jetson-Nano`**](https://github.com/domcross/DeepSpeech-for-Jetson-Nano)

## LLM
You can specify any OpenAI compatible endpoint, but to make everything run on device here's what you need to do:

> [!IMPORTANT]
Power supply might be a bottleneck, if the board goes down the easiest way is to switch to 5W mode by `sudo nvpmodel -n 1`

```bash
docker run -it \
    -p 8000:8000 \
    acerbetti/l4t-jetpack-llama-cpp-python:latest \
    /bin/bash -c \
    'python3 -m llama_cpp.server \
        --model $(huggingface-downloader TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/tinyllama-1.1b-chat-v1.0.Q5_K_M.gguf) \
        --model_alias tinyllama-1.1b-chat \
        --n_ctx 1024 \
        --n_gpu_layers 35 \
        --host 0.0.0.0 \
        --port 8000'
```

The above will start an outdated version of `llama.cpp` OpenAI compatible server running on `8000`, accelerated by Tegra X1 available on the board. `tinyllama-1.1b-chat` will be something around 5TPS.
**All the credit goes to [Running llama.cpp on the Jetson Nano](https://www.caplaz.com/jetson-nano-running-llama-cpp/)**

## Home Assistant and MCP

`docker-compose.yaml` runs Home Assistant, the supported Matter.js Server, and a third-party Home Assistant MCP server. It provides the MCP endpoint; connecting an LLM or MCP client is a separate step. The pinned images support `linux/arm64` and `linux/amd64`.

> [!WARNING]
> Matter Server requires a recent 64-bit Linux host with IPv6 and correctly configured multicast networking. The JetPack 4.6.1 environment above is not a validated Matter host. Host networking bypasses Docker port isolation, and the MCP server has a Home Assistant token plus read/write access to its configuration. Run this stack only on a trusted network and protect port `8099`.

1. Create your local configuration and data directories:

   ```bash
   cp .env.example .env
   mkdir -p ~/home_assistant_config ~/.matterjs-server
   sudo chown -R 1000:1000 ~/.matterjs-server
   ```

2. For a new Home Assistant installation, start only Home Assistant and its Matter dependency, then complete onboarding at `http://<host-ip>:8123`:

   ```bash
   docker compose up -d homeassistant
   ```

   The placeholder token in `.env` is not used because this command does not start the MCP service. Create a **new** long-lived access token from your Home Assistant profile. If a token was previously committed or shared, revoke it rather than reusing it here.
3. Edit `.env` and replace the placeholder `HA_TOKEN`. Optionally set a strong `HA_AGENT_KEY`; leaving it blank lets the MCP server generate one. For remote MCP clients, change `MCP_SERVER_URL` to this host's reachable address, for example `http://192.168.1.100:8099`.
4. Start and check the full stack:

   ```bash
   docker compose up -d
   docker compose ps
   docker compose logs -f
   ```

Home Assistant listens on port `8123`, Matter Server on `5580`, and MCP on `8099`. The MCP health endpoint is `http://localhost:8099/api/health`. MCP client configuration is generated in the Home Assistant configuration directory. When configuring the Matter integration manually, use `ws://localhost:5580/ws`; all three containers share the host network.

To enable Bluetooth commissioning on a host with BlueZ and adapter `hci0`, add `BLUETOOTH_ADAPTER: "0"` to the `matter-server` environment. The supplied `NOBLE_BINDINGS=dbus` setting keeps the container unprivileged. Matter networking and Thread requirements are documented in the [Matter.js Server deployment guide](https://github.com/matter-js/matterjs-server/blob/main/docs/docker.md).

For an existing Python Matter Server installation, stop the old container and back up its data before using the same directory. The new server migrates it once and needs UID/GID `1000` write access. Never run both servers against the same data. See the [migration guide](https://github.com/matter-js/matterjs-server/blob/main/docs/migration.md).

### Future ideas
* [ ] Bring [TTS](https://github.com/coqui-ai/TTS)
* [ ] Attach [HomeAssistant MCP](https://www.home-assistant.io/integrations/mcp_server/) to LLM and smart-home enable the board
* [ ] Try bringing [YoloE](https://docs.ultralytics.com/models/yoloe/) and via STT promptable segmentation

### Setup on the photo
- Jetson Nano 4GB, rev A02
- [Waveshare Jetson Nano Case](https://www.waveshare.com/wiki/Jetson_Nano_Case_(C))
- [Waveshare Audio Card for Jetson Nano](https://www.waveshare.com/wiki/Audio_Card_for_Jetson_Nano)
- And edit reference pic of the setup with the prompt of `Make it Ghibli style, keep the macro and overall aesthetics` :rage1:
