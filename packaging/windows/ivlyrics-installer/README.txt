Spotifast · ivLyrics 검색 화면 업데이트 / Windows x64
설치 리비전: 0.12.0-ivlyrics.2

사용: Spotifast-ivLyrics-SearchDiscovery-windows-x64-setup.exe 실행
기존 앱은 트레이에서도 완전히 종료한 후 설치하세요.
한국어 설치 화면, 사용자 계정 설치, 시작 메뉴 바로가기와 제거 기능을 제공합니다.

변경 사항
- 최근 검색 옆 전체 삭제
- 빈 검색 화면의 추천 노래 카드와 최근 들은 곡
- 추천 데이터가 없을 때 기존 재생 기록 또는 장르 검색
- 한국어 제목·장르 버튼과 한글 곡명 표시
기존 글꼴, 색상, 카드, 곡 목록 구성요소를 그대로 사용합니다.

소스
Spotifast-ivLyrics-source.zip과 deps-1.zip, deps-2.zip, deps-3.zip을
모두 같은 폴더에 풀면 전체 소스가 준비됩니다. BUILD-SOURCE.txt를 확인하세요.
재배포할 때는 네 개의 소스 ZIP과 라이선스 고지를 함께 제공하세요.

설치파일 재생성
검증 도구: NSIS 3.11 (Unicode)
포터블 ZIP을 빈 폴더에 풀고 아래처럼 실행합니다.
makensis.exe /DPAYLOAD="C:\payload" /DOUTPUT="C:\output\setup.exe" Spotifast-ivLyrics.nsi
.nsi와 uninstall-files.nsh를 같은 폴더에 두세요.
포함 파일을 바꾸면 SHA256SUMS와 uninstall-files.nsh도 갱신하세요.

설치 검증은 별도 설치 경로·등록 키·시작 메뉴 폴더를 사용했습니다.
파일 1,004개 일치, 바로가기, 제거, 사용자 추가 파일 보존을 확인했습니다.
실제 설치 폴더 패치도 파일 1,004개 일치를 확인했습니다.
디지털 서명은 적용하지 않았습니다.
