# 로컬 수정판의 라이선스와 재배포

이 문서는 2026-10-12에 보완한 ivLyrics AdaptiveText 수정판의 배포 안내입니다.
원본 Spotifast의 공식 릴리스가 아닙니다. 각 구성요소의 라이선스 전문이 우선합니다.

## Spotifast와 ivLyrics

원본 Spotifast는 MIT License이며, `Copyright (c) 2026 Carmine Paolino`와
허가·면책 조항을 `LICENSE`에 그대로 보존했습니다.
출처: <https://github.com/crmne/spotifast>.

`src/ivlyrics/alsong.rs`는 ivLyrics의 ALSong 구현을 Rust로 이식한 수정 코드이며
LGPL-2.1-only가 적용됩니다. 라이선스 전문은 `LICENSE` 및
`assets/ivlyrics/LICENSE.LGPL-2.1`에 있습니다. 이식 기준과 변경 기능은
`docs/ivlyrics.md`에 기록되어 있습니다. 출처: <https://github.com/ivLis-Studio/ivLyrics>.

수령자는 해당 코드와 앱 소스를 수정하고 재컴파일·재링크할 수 있습니다.
자신의 사용을 위한 수정이나 그 수정의 디버깅을 위한 역공학을 금지하는
추가 조건을 부과하지 않습니다. 기존 MIT 코드의 권리는 그대로 유지됩니다.

이 저장소는 앱 소스를 제공합니다. 의존성은 Cargo.lock으로 버전이 고정되며,
Cargo가 원본 저장소에서 내려받습니다. 소스만 공개하는 경우와 실행 파일을
배포하는 경우의 조건은 구분해야 합니다. 실행 파일을 배포한다면 수정·재링크에
필요한 대응 소스 제공 등 LGPL의 해당 조건을 별도로 충족해야 합니다.

## Pretendard JP Variable

앱에 내장된 글꼴은 **Pretendard JP Variable 1.3.9**입니다.

Copyright (c) 2021, Kil Hyung-jin (<https://github.com/orioncactus/pretendard>),
with Reserved Font Name Pretendard JP.

- 글꼴에는 **SIL Open Font License 1.1**이 적용됩니다. 앱의 MIT 라이선스로
  변경하거나 독점 라이선스를 덧붙이지 않습니다.
- `Pretendard-LICENSE.txt`를 실행 파일 ZIP에, 같은 전문을
  `assets/fonts/Pretendard-LICENSE.txt`로 소스 ZIP에 포함합니다.
  재배포할 때도 저작권 고지와 전문을 함께 유지하세요.
- 폰트 자체를 단독 판매하지 않습니다. 앱에 포함하거나 내장하는 것은 허용됩니다.
- 현재 파일은 공식 v1.3.9 원본과 동일하며 서브셋 제작·이름 변경·윤곽 수정 없이
  내장했습니다. 추후 폰트를 수정하면 예약 글꼴명 조건도 따라야 합니다.
- 저작권자의 이름을 수정판의 보증·추천·홍보에 사용하지 않습니다.

공식 원본:
<https://github.com/orioncactus/pretendard/tree/v1.3.9/packages/pretendard-jp>.
`PretendardJPVariable.ttf` SHA-256:
`bc1d77db0f3cb9c20986b8939aa8a28d5216552d777d082955da22f420535cbb`.

## 그 밖의 구성요소

`THIRD-PARTY-NOTICES.txt`와 `licenses/`에는 Cargo 의존성 및 함께 사용하는
폰트·아이콘의 라이선스와 저작권 고지를 제공합니다. 각 파일에 적용되는 별도
조건을 유지하세요. Cargo 의존성의 해당 소스 위치는 Cargo.lock과 THIRD-PARTY-NOTICES.txt에 기록되어 있습니다.
선택 가능한 라이선스는 priority-queue의 MPL-2.0, self_cell의 Apache-2.0을 사용합니다.

Spotify 상표·로고의 권리는 Spotify에 있습니다. 오픈소스 코드나 폰트의 라이선스가
그 상표의 사용 허가를 대신하지 않습니다. 기존 `ICON-NOTICE.md`도 보존합니다.

## 다시 빌드하기

`BUILD-SOURCE.txt`를 따르세요. Rust 1.98.0 및 Windows GNU용 C/C++
도구 체인은 별도로 설치해야 합니다. Cargo는 고정된 의존성을 네트워크로 내려받습니다. 컴파일러·Windows 시스템 구성요소는 별도로 설치해야 합니다.
재빌드 결과가 원본 EXE와 바이트 단위로 같다는 뜻은 아닙니다.
