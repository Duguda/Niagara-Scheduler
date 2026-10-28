# Niagara Scheduler — Claude 컨텍스트

새 대화를 시작하면 이 파일 하나만 읽고 시작한다.

## 개요
- 레포: [Duguda/Niagara-Scheduler](https://github.com/Duguda/Niagara-Scheduler) (upstream 리모트 = 원본)
- 원본: [ImgyeongLee/NoraSchedule](https://github.com/ImgyeongLee/NoraSchedule) v0.5.0 (MIT, Copyright 2026 Imgyeong Lee)
- 목적: 개인용 데스크톱 일정·생활 관리 앱을 내 생활 방식에 맞게 개량
- 방향: 불필요한 기능 제거 → 노트(마크다운 폴더)·모닝 브리프 연동 → 내 루틴(자격증, 러닝, 집중) 중심 기능 추가

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
  lib/obsidian.ts    마크다운(위키링크·콜아웃 등) → HTML, txt 표시
  pages/Notes.svelte 노트 페이지 (읽기·편집·새 노트·txt↔md)
  pages/Study.svelte 암기(인출 연습) 페이지
  lib/cloze.ts       굵게 → 빈칸 변환, 모드 판별
  lib/pages.svelte.ts 기능(페이지) 켜고 끄기, 위젯↔기능 연결, 꺼진 기능의 백그라운드 작업 정지
  lib/guide.svelte.ts / components/Guide.svelte  첫 실행 안내
  components/PageToggles.svelte  기능 아이콘 타일 (설정·안내 공용)
src-tauri/src/
  main.rs            앱 시작, 데이터 폴더 결정, 커맨드 등록
  db.rs              스키마 + 마이그레이션 + 공용 쿼리
  commands.rs        프론트에서 부르는 커맨드
  backup.rs          백업(zip) 내보내기/가져오기 (저장 .nia, 가져오기 .nia/.nora/.zip)
  tracker.rs         포커스 창 추적 (활동 기록)
  obsidian.rs        노트 폴더 목록·읽기·쓰기·만들기·확장자 바꾸기, 메모 → 노트 보내기
  brief.rs           모닝 브리프용 brief.json + brief.schema.json 생성
  storage.rs         저장 위치 결정 (설치 폴더 / 앱 데이터 폴더, data·brief 분류)
  cloze.rs           암기: #암기 태그 판별, 굵게 개수, ○/✕ 기록과 복습 상자
  reminders.rs       일정 알림
  recurrence.rs      반복 일정 규칙
docs/                이 문서들
```

## 데이터 위치
- 기준 폴더 아래 `data\`(DB `nora.sqlite3`, `images\`)와 `brief\`(`brief.json`, `brief.schema.json`)
- 기준 폴더: Windows 릴리스 빌드는 설치 폴더(쓰기 가능할 때), 아니면 `%APPDATA%\com.niagara.scheduler`. 이미 데이터가 있는 쪽 우선. 0.6 이하 루트 DB는 그 자리 유지 (`storage.rs`)
- 원본 Nora 데이터(`%APPDATA%\com.nora.schedule`)와는 분리. 옮길 때는 Nora에서 `.nora` 백업 내보내기 → Niagara에서 가져오기
- 노트 설정: settings 테이블 `notes.readDirs`(노트 폴더 JSON 목록), `notes.memoDir`(메모 보낼 폴더). 옛 `obsidian.vault`는 DB 열 때 목록으로 옮기고 비움 (`db.rs` migrate_vault_to_note_folders)
- 환경변수 `NORA_DATA_DIR`: 데이터 폴더를 직접 지정 (개발용, brief는 그 아래 `brief\`)

## 현재 상태
- v0.11.0: 노트 읽기·쓰기(.md/.txt), 새 노트, txt↔md 바꾸기, vault 설정 → 노트 폴더 목록, 옵시디언에서 열기 → 폴더에서 보기, 메모 자동 저장 제거, 백업 `.nia`
- v0.10.0: 메모·노트 통합 1단계 (메모 vault 자동 저장, 읽기 폴더 여러 개, 사이드바 '노트')
- v0.9.0: 기능 켜고 끄기(위젯·패널·작업 시간 추적까지), 첫 실행 안내(설정에서 다시 보기), 버전 표시 버그 수정
- v0.8.0: 암기 페이지(#암기 + 굵게 → 빈칸, ○/✕, 복습 상자), 일본어 글꼴 제거·페이지 지연 로딩, 꾸미기 설정으로 이동
- v0.7.0: 저장 구조 개편(설치 폴더 data·brief), brief.schema.json, 설정의 저장 위치 카드
- v0.6.x: 옵시디언 노트 페이지, 메모 → 옵시디언 보내기, 모닝 브리프 파일, Windows 자동 빌드
- 검증: `npm run check` 오류 0, `npm run build` 통과, `cargo test` 63개 통과, 브라우저 미리보기(Playwright)로 화면·뒤로 가기·링크 확인
- Windows CI 빌드: v0.7.0 성공. 실제 PC 확인은 아직
- 알파 테스트 화면: 예시 데이터 미리보기를 claude.ai 아티팩트로 공개(비공개 링크). `npm run preview:build` 결과를 올림
- GitHub: Duguda/Niagara-Scheduler main에 push (리모트 `origin`, 원작은 `upstream`)
- 공개 레포라 커밋 작성자는 `Duguda <74199426+Duguda@users.noreply.github.com>`만 사용. 실명·개인 이메일을 레포에 넣지 않는다

## 모닝 브리프 연결
- 예약 작업 "모닝 브리프"(평일 08:06 KST) 지시문에 Niagara 섹션을 추가함 (2026-10-28)
- 읽는 경로: 사용자 PC 파일 도구로 `brief\brief.json`(설치 폴더 안, 없으면 `com.niagara.scheduler\brief\`)을 직접 읽음. 구조는 `brief.schema.json`. 없으면 조용히 생략
- 쓰려면 데스크톱 앱에서 그 예약 작업의 "Require this computer"를 켜야 함 (아직 안 함). 실행 시각에 PC가 켜져 있어야 함

## 의사결정 로그
| 날짜 | 결정 | 채택/미채택 | 근거 |
|---|---|---|---|
| 2026-10-28 | 원본을 포크해서 개량 | 채택 | MIT라 수정·재배포 가능. 저작권 표시만 유지 |
| 2026-10-28 | TRPG 기능 제거 | 채택 | 사용하지 않음 |
| 2026-10-28 | 기존 DB의 `trpg_entries` 테이블은 삭제하지 않고 방치 | 채택 | 원본 앱과 DB를 공유 중이라 데이터 파괴 위험. 생성·마이그레이션 코드만 제거 |
| 2026-10-28 | 앱 이름 Niagara Scheduler, identifier `com.niagara.scheduler` | 채택 | 원본 브랜드와 구분, 원본 DB와 분리해 안전하게 스키마 변경 |
| 2026-10-28 | 내부 이름(`nora.sqlite3`, `noraimg:` 프로토콜, `nora.*` 저장 키, `NORA_DATA_DIR`)은 유지 | 채택 | 원본 백업을 그대로 가져올 수 있게 호환성 유지. 사용자에게 보이는 이름만 변경 |
| 2026-10-28 | 레포 공개 유지, 작성자·저작권 표기는 Duguda로 | 채택 | 공개 포크는 원작자에게 영향 없음. 실명·이메일 노출만 막음 |
| 2026-10-28 | 옵시디언은 읽기 전용 + 메모 내보내기(한 방향)부터 | 채택 | 양방향 동기화는 충돌 처리 위험이 큼. 노트 수정은 옵시디언에서 |
| 2026-10-28 | 모닝 브리프는 DB 직접 읽기 대신 `brief.json` 제공 | 채택 | 반복 일정 펼치기를 앱이 처리, 민감 데이터(일기·가계부·외주·추적)를 구조적으로 제외 |
| 2026-10-28 | Windows 설치 파일은 GitHub Actions Artifacts로만 배포, Release는 만들지 않음 | 채택 | 개인용. 공개 릴리스는 필요할 때 따로 결정 |
| 2026-10-28 | 구글 드라이브 경유(복사본 폴더) | 미채택 | 외부 서비스·용량에 기대지 않음. Niagara는 로컬에만 두고 Claude가 PC에서 직접 읽는 방향 |
| 2026-10-28 | 데이터를 설치 폴더에 용도별(data·brief)로 저장 | 채택 | 한 폴더만 허락하면 Claude가 읽을 수 있음. Tauri 언인스톨러는 설치한 파일만 지워 data가 남음 (AppData는 '앱 데이터 삭제' 체크 시 통째로 지워짐) |
| 2026-10-28 | 브리프 구조를 brief.schema.json으로 분리 제공 | 채택 | 읽는 쪽이 필드 뜻을 따로 불러옴. version 필드로 변경 구분, 테스트로 코드와 일치 보장 |
| 2026-10-28 | 암기 대상은 `#암기` 태그로만 고름 | 채택 | 굵게는 강조로도 흔히 써서 목록이 지저분해짐. 태그는 노트 파일에 저장, Niagara는 기록만 |
| 2026-10-28 | 암기 목록은 폴더별 묶음 | 채택 | 하위 태그 분류는 보기 복잡. 기존 vault 폴더 체계를 그대로 씀 |
| 2026-10-28 | 그래프 뷰 | 보류 | 무겁고 '오늘 뭘 외울지'에는 불편. 설정에서 켜는 확장 기능으로 검토 |
| 2026-10-28 | 복습 간격 1·2·4·8·16일 (라이트너 상자) | 임시 채택 | 사용자 참고 자료(인출 연습 영상) 확인 후 조정. 값은 cloze.rs BOX_INTERVALS 한 곳 |
| 2026-10-28 | 일본어 글꼴 번들 제거 | 채택 | 화면 파일의 절반 이상. 사용자는 일본어를 쓰지 않음 |
| 2026-10-28 | 페이지 숨기기 → 기능 끄기로 의미 확장 | 채택 | 범용 기능은 두되 원하면 끔. 끄면 위젯·패널·백그라운드 작업까지 멈춰야 가벼움에 의미가 있음 |
| 2026-10-28 | 첫 실행 안내는 한 번만, 설정에서 다시 보기 | 채택 | 1회성으로 사라지되 필요할 때 다시 볼 수 있게 |
| 2026-10-28 | 옵시디언 로고를 버튼 아이콘으로 그리기 | 미채택 | 다른 회사의 상표. 일반 아이콘(보석·연필)으로 대신 |
| 2026-10-28 | 메모 통합을 2단계로: 먼저 DB 메모 → vault 자동 사본 | 채택 | 메모 화면 재작성은 큰 공사. 사본 방식으로 옵시디언에서 바로 보이게 먼저 해결, 사용량 고려 |
| 2026-10-28 | 읽기 폴더 여러 개 (vault + 추가 폴더) | 채택 | 강의 자료 등 vault 밖 폴더도 읽기. 쓰기는 vault 메모 폴더 하나 |
| 2026-10-28 | 메모를 파일 원본으로 바꾸는 2단계 | 미채택 | 메모(Niagara)와 노트(옵시디언)는 다른 기능. 현재 방식(DB 원본 + vault 사본)으로 충분 |
| 2026-10-28 | 노트 폴더의 .md·.txt를 Niagara에서 편집 | 채택 | 노트 기능이면 쓰기도 돼야 함 (사용자). 다른 프로그램과의 충돌은 수정 시각 비교로 막음 |
| 2026-10-28 | vault 설정을 노트 폴더 목록으로 합침, "옵시디언에서 열기" 제거 | 채택 | 옵시디언에 매몰되지 않게. 문법도 마크다운으로 부름 |
| 2026-10-28 | 메모를 vault에 자동 저장 | 제거 | 메모와 노트는 다른 기능. "노트로 보내기"만 남김 |
| 2026-10-28 | txt ↔ md 바꾸기는 확장자만 변경 | 채택 | 내용은 그대로 두는 게 안전하고 단순 |
| 2026-10-28 | 백업 저장 확장자 `.nora` → `.nia`, 가져오기는 둘 다 | 채택 | Nora와 Niagara를 같이 쓰는 사람이 파일을 헷갈리지 않게 (사용자) |
| 2026-10-28 | 공유되던 번역 키 `trpg.year/noMatch/playUnit` → `common.year/noMatch/timesUnit` | 채택 | 일기·외주 화면이 계속 사용 |

## 개선 로드맵
2. 실제 Windows 앱으로 노트 편집·브리프·암기·기능 끄기 확인 (CI 설치 파일)
3. 그래프 뷰를 설정에서 켜는 확장 기능으로 (폴더 구조 시각화)
4. 암기: 브리프에 '오늘 복습 N개' 넣기, AI 문제는 quiz\ 폴더 계약으로, 복습 간격은 사용자 자료 확인 후 조정
5. 아이디어(구현 보류): 구글 캘린더 등 외부 캘린더 가져오기·쓰기 연동

## 메모와 노트 (2026-10-28 결정)
- **메모** = Niagara DB에 저장되는 기능. 원본은 DB. 파일로 남기고 싶을 때만 "노트로 보내기"(한 방향 사본)
- **노트** = 노트 폴더(여러 개)의 `.md`·`.txt` 파일. Niagara에서 읽고 쓰기 모두 가능 = 사용자가 말한 "양방향"의 뜻 (메모↔파일 동기화가 아님)
- vault는 특별 취급 없이 노트 폴더 중 하나. 옵시디언 전용 기능(옵시디언에서 열기)은 두지 않음, 문법도 "마크다운"으로 부름
- txt ↔ md는 확장자만 바꾸는 버튼으로 해결 (내용 변환 없음)
- 이전 설계안(메모를 파일 원본으로 바꾸기, 메모 vault 자동 저장)은 미채택·제거

## 미결정 사항
- 아이콘 교체 여부 (아직 원본 아이콘)
- 외주(커미션)·북마크 등 다른 기능도 뺄지
- 추천 기능 중 무엇을 할지
