# Spotifast · ivLyrics

Spotifast의 로컬 수정판 소스입니다. 원본 프로젝트의 공식 배포본은 아닙니다.

## 빌드

Rust 1.98.0과 대상 플랫폼의 C/C++ 빌드 도구가 필요합니다.

```sh
cargo build --release --locked --no-default-features --bin spotifast
```

Cargo가 `Cargo.lock`에 고정된 의존성을 내려받습니다. 별도의 의존성 ZIP을
먼저 풀 필요는 없습니다. Windows GNU 도구 설정은 [BUILD-SOURCE.txt](BUILD-SOURCE.txt)를 참고하세요.

## 라이선스

- **Spotifast — MIT**: Copyright (c) 2026 Carmine Paolino. [원문](LICENSE)
- **Pretendard JP — OFL 1.1**: Copyright (c) 2021 Kil Hyung-jin. 수정하지 않은 원본 글꼴을 내장합니다. [원문](assets/fonts/Pretendard-LICENSE.txt)
- **ivLyrics에서 이식한 코드 — LGPL-2.1-only**: [원문](assets/ivlyrics/LICENSE.LGPL-2.1)

저작권 고지와 라이선스 전문을 유지하세요. 실행 파일을 재배포할 때는
대응 소스 제공 등 추가 조건도 따라야 합니다.
[다른 구성요소의 고지](THIRD-PARTY-NOTICES.txt) · [재배포 안내](docs/local-distribution-licenses.ko.md)

## 원본 프로젝트

[Spotifast](https://github.com/crmne/spotifast) · [ivLyrics](https://github.com/ivLis-Studio/ivLyrics) · [Pretendard](https://github.com/orioncactus/pretendard)

원본 저장소의 GitHub 자동화 설정은 `.github/upstream-workflows/`에 참고용으로
보존했습니다. 이 저장소에서 자동 실행되지는 않습니다.
