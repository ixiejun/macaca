# AI Speech Pack

`pack.ai.speech.v1` provides provider-neutral speech-to-text, text-to-speech,
voice catalog inspection, speech translation, and timing alignment. The pack is
descriptor-only until a serviceized speech provider is registered.

Applications use audio handles, voice refs, transcript refs, and artifact refs;
they do not put raw audio, generated speech bytes, voice biometrics, or native
provider payloads into OS observability surfaces.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.ai.speech.v1"]
```

Optional declarations degrade with `ai_speech_provider_not_installed`.

## Permission Scopes

- `ai.speech.recognize`: speech-to-text and streaming transcription.
- `ai.speech.synthesize`: text-to-speech and voice catalog usage.
- `ai.speech.translate`: speech translation and alignment.

## Commands

- `speech.speech_to_text`: transcribes bounded audio refs.
- `speech.text_to_speech`: synthesizes speech from a text ref and voice ref.
- `speech.list_voices`: returns `VoiceDescriptor` rows.
- `speech.translate_speech`: translates speech through provider-neutral refs.
- `speech.align_timing`: returns timing alignment refs.

## DTOs And Results

Core DTOs include `SpeechAudioInput`, `SpeechStreamFrame`,
`TranscriptSegment`, `VoiceDescriptor`, `SpeechSynthesisRequest`,
`SpeechSynthesisResult`, and `SpeechAlignment`. Statuses include success,
partial, denied, unavailable, unsupported, conflict, quota exceeded, stream
cancelled, unsupported voice, audio too long, alignment unavailable, and
provider failure.

## Examples

Minimal declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.speech.v1"]
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.speech.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_speech_provider_not_installed"
}
```

Canonical synthesis command payload:

```json
{
  "subject_ref": "audio:target",
  "parameters": {
    "request_ref": "speech-synthesis-ref",
    "text_ref": "text-ref",
    "voice_ref": "voice-ref"
  },
  "idempotency_key": "speech-synthesis-key"
}
```

## Trace And Audit

Trace evidence may include audio refs, transcript segment refs, voice refs,
alignment refs, locale tags, duration counters, and bounded error codes. It
must not include raw audio, generated speech bytes, transcript text,
credentials, voice biometric data, or provider payloads.

## Provider Replacement

Provider classes include `speech-recognition`, `speech-synthesis`,
`remote-service`, `mock`, and `unavailable`. Runtime composition roots own
adapter wiring; SDK helpers only build traced service-call commands.
