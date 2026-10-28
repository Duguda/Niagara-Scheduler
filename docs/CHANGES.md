# 변경 기록

## v0.7.0 — 2026-10-28
- **저장 구조 개편** (`storage.rs`): 한 기준 폴더 아래를 용도별로 나눔
  - `data\` 앱 내부용 (DB, 이미지) · `brief\` 바깥 도구용 (`brief.json`, `brief.schema.json`)
  - Windows 릴리스 빌드는 설치 폴더를 기준으로 씀 (쓰기 가능할 때). 언인스톨러는 자기가 설치한 파일만 지우므로 두 폴더는 삭제·업데이트에도 남음
  - 설치 폴더에 쓸 수 없으면 앱 데이터 폴더(`%APPDATA%\com.niagara.scheduler`) 사용. 이미 데이터가 있는 쪽을 항상 우선해 위치가 바뀌지 않음. 0.6 이하의 루트 DB도 그 자리에서 계속 사용
- **`brief.schema.json`**: 브리프 구조 설명(JSON Schema, 필드 뜻은 한국어)을 따로 저장. 테스트로 실제 출력과 일치를 보장
- 설정의 "모닝 브리프 파일" 카드를 **"저장 위치"** 카드로 확장: 앱 데이터, 모닝 브리프, 옵시디언 노트 폴더, 보낸 메모 폴더를 한곳에서 보고 폴더 열기
- 구글 드라이브 복사본 경로는 넣지 않기로 함
- 테스트 6개 추가 (Rust 56개)

## v0.6.1 — 2026-10-28
- 전체 코드 리뷰로 찾은 버그 수정
  - 브리프: 전날 밤 자정에 끝나는 일정이 오늘 일정에 섞이던 문제
  - 옵시디언: 뒤로 가기를 누르면 이동 기록이 전부 지워지던 문제
  - 옵시디언: 제목의 대소문자만 바뀐 메모를 다시 보내면 새 파일이 생기던 문제 (Windows·macOS)
- 옵시디언: 일반 마크다운 링크(`[글](other.md)`)도 앱 안에서 이동, vault 안 이미지는 첨부 칩으로 표시, 체크리스트 글머리표 정리
- GitHub Actions: `main`에 push하면 Windows 설치 파일 자동 빌드 (`.github/workflows/build.yml`, 결과는 Actions 실행의 Artifacts)
- 테스트 1개 추가 (Rust 50개)

## v0.6.0 — 2026-10-28
- **옵시디언 노트 페이지** 추가 (사이드바 "옵시디언")
  - vault 폴더 선택 → 노트 목록(폴더 필터, 검색, 최근 수정 순), 읽기 전용 보기
  - 위키링크 이동·뒤로 가기, 콜아웃, 하이라이트, 프런트매터 태그, 첨부 표시
  - "옵시디언에서 열기" (`obsidian://open?path=`), 열기 권한을 `obsidian://`로 한정
  - 백엔드 `obsidian.rs`: 경로가 vault 밖으로 나가지 못하게 검사, `.obsidian` 등 점 폴더 제외, 노트 2MB·목록 5000개 제한
- **메모 → 옵시디언 보내기** (메모 보기 화면 버튼, 우클릭 메뉴)
  - 프런트매터 `niagara_memo: <id>`로 같은 메모를 다시 보내면 갱신, 제목이 바뀌면 파일 이름도 바꿈
  - 사용자가 쓴 노트는 덮어쓰지 않고 번호를 붙임
- **모닝 브리프 파일** `brief.json` (데이터 폴더, 시작 시 + 5분마다 갱신)
  - 오늘·내일 일정(반복 일정 펼침), 밀린 할 일, 오늘 할 일, 60일 이내 디데이, 어제 집중 시간
  - 일기·가계부·외주·작업 시간 추적은 넣지 않음
  - 설정에 위치 표시와 "지금 갱신" 버튼
- 사이드바·오버레이 패널 로고 이름 Niagara로 변경
- README를 "Duguda 방식의 Nora"로 재작성 (차이점 표, 새 기능, Nora에서 옮겨 오기)
- 테스트 8개 추가 (Rust 49개 통과)

## v0.5.1 — 2026-10-28
- 원본 v0.5.0(ImgyeongLee/NoraSchedule)에서 포크
- TRPG 기능 제거
  - 프론트: `pages/Trpg.svelte`, `components/TrpgReport.svelte`, `lib/trpgRoles.ts` 삭제, 메뉴·라우트·타입·API·목업·번역 정리
  - 백엔드: `trpg.rs` 삭제, 커맨드 4개 등록 해제, 스키마와 마이그레이션 제거, 이미지 참조 쿼리와 백업 개수에서 제외
  - 기존 DB의 `trpg_entries` 테이블은 건드리지 않음 (데이터 보존)
- 공유 번역 키 이름 변경: `trpg.year` → `common.year`, `trpg.noMatch` → `common.noMatch`, `trpg.playUnit` → `common.timesUnit`
- README에서 TRPG 설명 제거
- `docs/` (Claude.md, CHANGES.md) 추가
- 이름 변경: Nora Schedule → **Niagara Scheduler**
  - productName, 창 제목, 알림 제목, 번역 문구(영어·한국어·일본어), 백업 기본 파일명
  - identifier `com.nora.schedule` → `com.niagara.scheduler` (데이터 폴더 분리)
  - 패키지명 `niagara-scheduler`, 버전 0.5.1
  - 내부 이름과 `.nora` 백업 형식은 원본 호환을 위해 유지
- README에 원본 출처 표기, LICENSE에 수정자 저작권 줄 추가
