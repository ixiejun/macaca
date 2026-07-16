# Change: Add Media Audio Pack

## Why

Developers need `pack.media.audio.v1` as an industrial audio capability for
audio provider inspection, audio import/opening, metadata inspection, waveform
and loudness inspection, transcoding, trimming/segmenting, mixing, normalization,
filtering, format/container conversion, text-to-speech synthesis, safe export,
artifact management, and replay diagnostics. It must not be a thin wrapper
around FFmpeg, GStreamer, Web Audio, libsndfile, OpenAI TTS, ElevenLabs, Google
Text-to-Speech, Amazon Polly, or one codec library.

Audio can contain private conversations, voice biometrics, copyrighted music,
regulated calls, identity data, legal/medical recordings, and generated voices.
Processing can leak speaker identity, alter evidence, publish copyrighted
content, or synthesize misleading speech. Macaca must therefore expose audio
operations only through provider-neutral typed service commands with
permission, policy, entitlement, resource, approval, voice/speaker safety,
copyright and consent metadata, artifact retention, trace, audit, health,
snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- FFmpeg exposes industrial audio demuxing/muxing, codec conversion, filters,
  resampling, trimming, loudness normalization, mixing, metadata, and container
  operations. Reference: https://ffmpeg.org/documentation.html
- GStreamer exposes graph/pipeline-based media processing, elements, pads,
  encoders/decoders, mixers, resamplers, filters, and streaming pipelines.
  Reference: https://gstreamer.freedesktop.org/documentation/
- Web Audio API exposes browser audio graphs, sources, gain, filters, mixers,
  buffers, analyzers, and offline rendering concepts. Reference:
  https://www.w3.org/TR/webaudio/
- libsndfile exposes local audio file read/write support for common PCM-oriented
  formats and format/subtype inspection. Reference:
  https://libsndfile.github.io/libsndfile/api.html
- OpenAI audio speech, ElevenLabs Text-to-Speech, Google Cloud Text-to-Speech,
  and Amazon Polly provide text-to-speech synthesis baselines with voice/model,
  format, latency, streaming, and safety constraints. References:
  https://platform.openai.com/docs/guides/text-to-speech,
  https://elevenlabs.io/docs/api-reference/text-to-speech,
  https://cloud.google.com/text-to-speech/docs/reference/rest, and
  https://docs.aws.amazon.com/polly/latest/dg/API_SynthesizeSpeech.html

Macaca maps these supplier concepts into provider-neutral audio scope,
provider capability, audio handle, stream metadata, codec/container metadata,
waveform summary, loudness report, segment, filter operation, mix graph,
voice synthesis plan, export plan, artifact handle, provider capability,
version/freshness metadata, and diagnostics DTOs. Concrete FFmpeg, GStreamer,
Web Audio, libsndfile, OpenAI, ElevenLabs, Google TTS, Polly, storage,
moderation, and export providers stay behind replaceable providers.

## What Changes

- Add provider-neutral `pack.media.audio.v1` under the `media` family.
- Define command namespace `audio.*` for:
  - provider capability inspection
  - audio import/open and metadata inspection
  - waveform, loudness, silence, duration, channels, sample rate, codec, and
    container inspection
  - transcode/export planning and requests
  - trim, segment, split, concatenate, normalize, resample, filter, and mix
    planning and requests
  - text-to-speech synthesis planning and requests
  - artifact handle resolution, snapshots, and replay
- Define DTOs for audio scope, provider capability, audio handle, audio
  metadata, stream metadata, waveform summary, loudness report, segment,
  filter operation, mix source, mix graph, synthesis voice, synthesis plan,
  export plan, artifact handle, event cursor, and diagnostics.
- Define permission scopes, policy defaults, audio/artifact scopes, voice/
  speaker safety, consent/copyright metadata, generated-voice provenance,
  approval rules, resource/entitlement behavior, SDK discovery, developer
  documentation, trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/media/audio.md` before implementation completion.

## Impact

- Affected specs: `pack-media-audio`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, audio media service
  provider or unavailable provider, runtime-host provider adapters, artifact/
  redaction/moderation support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete FFmpeg/GStreamer/Web Audio/libsndfile/OpenAI/
  ElevenLabs/Google TTS/Polly/storage/moderation/export provider implementation
  in this proposal; no transcription, speech recognition, diarization, voice
  cloning, music generation, podcast workflow, call-center workflow, or
  application-specific audio editor logic; no provider-name, model-name,
  voice-name, codec-name routing in OS layers beyond declarative descriptor
  data; no raw credentials, raw prompts, private recordings, speaker biometric
  features, raw generated audio, raw provider payloads, manifests, package
  bytes, private keys, signatures, or unbounded PCM/sample data in
  observability; no SDK/shell/kernel provider construction; no fake success
  when provider, codec, container, voice/model, safety, permission,
  entitlement, approval, resource, or host support is absent.
