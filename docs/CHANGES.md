# 변경 기록

## v0.5.1 — 2026-10-28
- 원본 v0.5.0(ImgyeongLee/NoraSchedule)에서 포크
- TRPG 기능 제거
  - 프론트: `pages/Trpg.svelte`, `components/TrpgReport.svelte`, `lib/trpgRoles.ts` 삭제, 메뉴·라우트·타입·API·목업·번역 정리
  - 백엔드: `trpg.rs` 삭제, 커맨드 4개 등록 해제, 스키마와 마이그레이션 제거, 이미지 참조 쿼리와 백업 개수에서 제외
  - 기존 DB의 `trpg_entries` 테이블은 건드리지 않음 (데이터 보존)
- 공유 번역 키 이름 변경: `trpg.year` → `common.year`, `trpg.noMatch` → `common.noMatch`, `trpg.playUnit` → `common.timesUnit`
- README에서 TRPG 설명 제거
- `docs/` (Claude.md, CHANGES.md) 추가
