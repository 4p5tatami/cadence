# Audio test fixtures

`nonzero-origin.ogg` is an original synthetic stereo test tone, generated locally
with VLC's Vorbis encoder (192 kb/s) and Ogg muxer. It contains no recorded music.
The source was 12 seconds of signed 16-bit PCM at 44,100 Hz:

```text
left[i]  = trunc(16000 * sin(i * 0.063))
right[i] = trunc(16000 * sin(i * 0.097))
```

The nonzero initial granule timestamp reproduces Rodio 0.21.1 rejecting a seek
to zero and offsetting later seeks. The regression compares decoded output
with a sequential reference. FLAC fixtures are generated directly in Rust tests.
