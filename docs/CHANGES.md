# 변경 기록

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
