# Niagara Scheduler — Claude 컨텍스트

새 대화를 시작하면 이 파일 하나만 읽고 시작한다.

## 개요
- 레포: [Duguda/Niagara-Scheduler](https://github.com/Duguda/Niagara-Scheduler) (upstream 리모트 = 원본)
- 원본: [ImgyeongLee/NoraSchedule](https://github.com/ImgyeongLee/NoraSchedule) v0.5.0 (MIT, Copyright 2026 Imgyeong Lee)
- 목적: 개인용 데스크톱 일정·생활 관리 앱을 내 생활 방식에 맞게 개량
- 방향: 불필요한 기능 제거 → 옵시디언·모닝 브리프 연동 → 내 루틴(자격증, 러닝, 집중) 중심 기능 추가

## 기술 스택
- 데스크톱 셸: Tauri 2 (Rust)
- 프론트: Svelte 5 + TypeScript + Vite (개발 서버 1420)
- 저장: SQLite 단일 파일 (rusqlite, WAL 모드)
- 메모 에디터: Tiptap + Markdown 확장 → **메모 본문은 Markdown 텍스트로 저장됨**
- 플러그인: notification, autostart, tray, window-state, dialog, opener, single-instance

## 파일 구조 (핵심만)
```
src/                 프론트 (Svelte)
  pages/             페이지별 화면 (Calendar, Todos, Memos, Diary, Pomodoro …)
  widgets/           홈 화면 위젯
  components/        공용 컴포넌트 (RichEditor 등)
  lib/api.ts         Rust 커맨드 호출 + 타입 정의
  lib/i18n*.ts       번역 (영어·한국어는 i18n.svelte.ts, 일본어는 i18n.ja.ts)
  lib/mock.ts        브라우저 미리보기용 가짜 데이터
src-tauri/src/
  main.rs            앱 시작, 데이터 폴더 결정, 커맨드 등록
  db.rs              스키마 + 마이그레이션 + 공용 쿼리
  commands.rs        프론트에서 부르는 커맨드
  backup.rs          .nora 백업(zip) 내보내기/가져오기
  tracker.rs         포커스 창 추적 (활동 기록)
  reminders.rs       일정 알림
  recurrence.rs      반복 일정 규칙
docs/                이 문서들
```

## 데이터 위치
- Windows: `%APPDATA%\com.niagara.scheduler\nora.sqlite3` (+ `-wal`, `-shm`), 이미지는 `images\`
- 원본 Nora 앱 데이터(`%APPDATA%\com.nora.schedule`)와는 분리됨. 옮길 때는 원본에서 `.nora` 백업 내보내기 → Niagara에서 가져오기
- 환경변수 `NORA_DATA_DIR`로 다른 폴더 지정 가능 (개발 시 원본 데이터 보호용으로 사용할 것)

## 현재 상태
- v0.5.1: TRPG 기능 제거, 이름을 Niagara Scheduler로 변경
- 검증: `npm run check` 오류 0, `npm run build`, `cargo test` 통과 여부는 CHANGES.md 참고
- identifier `com.niagara.scheduler` → 원본 앱과 별도 데이터 폴더

## 의사결정 로그
| 날짜 | 결정 | 채택/미채택 | 근거 |
|---|---|---|---|
| 2026-10-28 | 원본을 포크해서 개량 | 채택 | MIT라 수정·재배포 가능. 저작권 표시만 유지 |
| 2026-10-28 | TRPG 기능 제거 | 채택 | 사용하지 않음 |
| 2026-10-28 | 기존 DB의 `trpg_entries` 테이블은 삭제하지 않고 방치 | 채택 | 원본 앱과 DB를 공유 중이라 데이터 파괴 위험. 생성·마이그레이션 코드만 제거 |
| 2026-10-28 | 앱 이름 Niagara Scheduler, identifier `com.niagara.scheduler` | 채택 | 원본 브랜드와 구분, 원본 DB와 분리해 안전하게 스키마 변경 |
| 2026-10-28 | 내부 이름(`.nora` 백업 확장자, `nora.sqlite3`, `noraimg:` 프로토콜, `nora.*` 저장 키, `NORA_DATA_DIR`)은 유지 | 채택 | 원본 백업을 그대로 가져올 수 있게 호환성 유지. 사용자에게 보이는 이름만 변경 |
| 2026-10-28 | 공유되던 번역 키 `trpg.year/noMatch/playUnit` → `common.year/noMatch/timesUnit` | 채택 | 일기·외주 화면이 계속 사용 |

## 개선 로드맵
1. 옵시디언 vault 연동 (메모 ↔ 노트) — 1순위
2. 모닝 브리프 연동 (오늘 일정·할 일·D-day를 브리프에 공급)
3. 그 외 추천 기능은 아래 "미결정 사항"에서 선택

## 미결정 사항
- 아이콘 교체 여부 (아직 원본 아이콘)
- 외주(커미션)·북마크 등 다른 기능도 뺄지
- 옵시디언 연동 방식: 읽기 전용 / 단방향 가져오기 / 양방향 동기화
- 추천 기능 중 무엇을 할지
