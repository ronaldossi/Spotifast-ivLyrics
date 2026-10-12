# Bundled interface font

The regular, medium, semibold and bold UI families use **Pretendard JP Variable
1.3.9**, including Korean, Japanese and Latin lyric text. This face includes
Hangul as well as Japanese kana and kanji; no system font installation or runtime
font download is required. The existing internal family names are retained for
callers of the shared theme helpers. Emoji and other-script fallbacks remain.

Source: [orioncactus/pretendard v1.3.9](https://github.com/orioncactus/pretendard/tree/v1.3.9/packages/pretendard-jp).
The unmodified font is distributed under the SIL Open Font License 1.1; see
[Pretendard-LICENSE.txt](Pretendard-LICENSE.txt).

Keep that copyright notice and the full OFL text with every binary or source
redistribution. The font remains OFL-1.1 licensed separately from the app code.
It must not be sold on its own. Modified font versions must respect the reserved
font name; this build embeds the original font without modification.
On 2026-10-12, the font and license were compared with the official v1.3.9 files:
the font matched byte for byte, and the license text matched after normalizing
line endings. Font SHA-256:
`bc1d77db0f3cb9c20986b8939aa8a28d5216552d777d082955da22f420535cbb`.

Native operating-system menus use the OS font. Classic Winamp skins retain their
own bitmap lettering. Inter's license is retained for existing native integrations.
