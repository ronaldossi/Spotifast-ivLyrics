# Spotifast · ivLyrics

Spotifast의 로컬 수정판 소스입니다. 원본 프로젝트의 공식 배포본은 아닙니다.

## 다운로드와 검색 화면 업데이트

[최신 Windows 설치파일 및 포터블 ZIP](https://github.com/ronaldossi/Spotifast-ivLyrics/releases/latest)

- 최근 검색 기록 전체 삭제
- 빈 검색 화면의 추천 노래와 최근 들은 곡
- 기존 화면 스타일을 유지한 한국어 제목·장르 버튼

추천을 가져오지 못하면 사용 가능한 재생 기록이나 장르 검색을 표시합니다.
[검색 화면 안내](docs/search-discovery.md)를 참고하세요.

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
