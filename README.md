# Niagara Scheduler

**Duguda 방식의 Nora.**

[Nora Schedule](https://github.com/ImgyeongLee/NoraSchedule)을 제 입맛대로 바꾸는 프로젝트예요. 공부하고 기록하고 계획하는 제 하루에 맞게 다시 짠 데스크톱 스케줄러예요.
Nora의 차분한 디자인과 탄탄한 기본기는 그대로 두고, 제가 쓰지 않는 건 덜어 내고, 제 도구들과 이어지는 길을 더했어요.

모든 데이터는 내 컴퓨터에만 저장돼요.

> 원작: **Nora Schedule** by Imgyeong Lee (MIT). Niagara는 원작과 별개의 개인 프로젝트이고, 원작자가 관리하거나 지원하지 않아요. 자세한 내용은 [출처와 감사](#출처와-감사)를 보세요.

## Nora와 무엇이 다른가

| | Nora Schedule | Niagara Scheduler |
|---|---|---|
| 방향 | 누구나 쓰는 일상 스케줄러 | 공부·자격증·운동 루틴 중심의 개인 스케줄러 |
| 옵시디언 | — | vault 노트를 앱 안에서 읽기, 메모를 vault로 보내기 |
| 암기 | — | `#암기` 노트의 굵게를 빈칸으로, ○/✕ 자기 채점과 복습 간격 |
| 가벼움 | 일본어 글꼴 포함 | 일본어 글꼴 제거, 페이지는 열 때 불러오기 (화면 파일 절반 이하) |
| 꾸미기 | 사이드바 버튼 | 설정 화면 안 |
| 기능 끄기 | 사이드바에서 숨기기 | 기능 단위로 끄기 (위젯·패널·백그라운드 작업까지) |
| 처음 실행 | — | 시작 안내 (설정에서 다시 보기) |
| 모닝 브리프 | — | 오늘의 요약 파일(`brief.json`)을 자동으로 남김 |
| TRPG 기록 | 있음 | 뺐어요 |
| 데이터 폴더 | `%APPDATA%\com.nora.schedule` | 설치 폴더의 `data\` (Nora와 따로 저장) |
| 백업 파일 | `.nora` | `.nora` 그대로 (Nora 백업을 바로 가져올 수 있어요) |

버전별로 바뀐 내용은 [docs/CHANGES.md](docs/CHANGES.md)에 있어요.

## Niagara에서 더한 것

### 옵시디언 노트
- 사이드바의 **옵시디언** 페이지에서 vault 폴더를 한 번 고르면, 노트를 앱 안에서 바로 읽어요.
- 폴더별 보기, 제목·경로 검색, 최근 수정 순 정렬
- 옵시디언 문법 지원: `[[위키링크]]` (눌러서 이동, 뒤로 가기), `> [!tip]` 콜아웃, `==하이라이트==`, 프런트매터 태그, `![[첨부]]` 표시
- **읽기 전용**이에요. vault의 노트를 고치거나 지우지 않아요. 고칠 땐 "옵시디언에서 열기"로 넘어가요.
- 메모 페이지의 💎 버튼이나 우클릭 메뉴로 **메모를 vault로 보내기**: 지정한 폴더(기본 `Niagara/`)에 `.md`로 저장해요. 같은 메모를 다시 보내면 그 노트를 갱신하고, 직접 쓴 노트와 이름이 겹치면 덮어쓰지 않고 번호를 붙여요.

### 쓰는 기능만 켜기
- **설정 → 기능**에서 아이콘을 눌러 기능을 켜고 꺼요. 처음 실행할 때 나오는 안내에서도 고를 수 있어요.
- 꺼 둔 기능은 사이드바, 한눈에 보기, 오버레이 패널에서 빠지고 **뒤에서 돌던 작업도 멈춰요.** 예를 들어 작업 시간을 끄면 활성 창을 읽던 추적이 쉬어요. 데이터는 그대로 남아서 다시 켜면 돌아와요.

### 시작 안내
- 처음 켜면 5단계 안내가 나와요: 언어 → 쓸 기능 → 옵시디언 연결 → 암기 사용법 → 저장 위치.
- 한 번 마치거나 건너뛰면 다시 안 떠요. **설정 → 정보 → 시작 안내 다시 보기**로 언제든 볼 수 있어요.

### 암기 (인출 연습)
- 노트나 메모에 **`#암기`** 태그를 달고, 외울 부분을 **`**굵게**`** 쓰면 끝이에요. 강조하려고 굵게만 쓴 노트는 목록에 안 나와요.
- 목록은 폴더별로 묶여요. 폴더 안에서는 오늘 복습할 것과 약점이 많은 노트가 위로 와요.
- 굵게 쓴 부분이 가려져요. **먼저 답을 떠올리고**, 눌러서 확인한 다음 **○ 또는 ✕로 스스로 채점**해요. 다시 읽기보다 꺼내보기가 기억에 오래 남는다는 인출 연습 방식이에요.
- 키보드만으로 빠르게: `Space` 펼치기 · `1` ○ · `2` ✕, 채점하면 다음 빈칸으로 넘어가요.
- 빈칸마다 기록이 남아요. 맞히면 복습 간격이 1 → 2 → 4 → 8 → 16일로 늘고, 틀리면 다음 날 다시 나와요. **오늘 복습**과 **약점**만 골라 풀 수도 있어요.

```markdown
# 네트워크관리사 2급 #암기
라우팅을 담당하는 계층은 **네트워크 계층**이다.
DNS 기본 포트: **53**
```

### 저장 위치

Niagara는 파일을 한 폴더 아래에 용도별로 나눠서 저장해요.

```
Niagara Scheduler\        ← 설치 폴더
├─ data\                  ← 앱 데이터: 데이터베이스, 이미지 (Niagara만 고쳐요)
└─ brief\                 ← 바깥 도구용: brief.json, brief.schema.json
```

- 설치 폴더에 쓸 수 있으면(예: `D:\Program Files\Niagara Scheduler`) 거기에 저장해요. 앱을 지우거나 업데이트해도 `data`와 `brief`는 남아요.
- 쓸 수 없는 곳(`C:\Program Files` 등)에 설치했다면 `%APPDATA%\com.niagara.scheduler` 아래에 같은 구조로 저장해요.
- 옵시디언 노트 폴더는 vault 그대로 두고 읽기만 해요. 모든 위치는 **설정 → 저장 위치**에서 한눈에 보고 열 수 있어요.

### 모닝 브리프 파일
앱이 켜져 있는 동안 5분마다 `brief\brief.json`을 갱신해요. 각 필드의 뜻은 같은 폴더의 `brief.schema.json`에 있어서, 읽는 쪽이 구조를 따로 불러올 수 있어요. 아침 브리프, 스크립트, 다른 AI 도구가 데이터베이스 구조를 몰라도 오늘 하루를 읽을 수 있게 하려는 거예요.

```json
{
  "date": "2026-10-28",
  "today":    [{ "title": "네트워크관리사 스터디", "start": "19:00", "end": "21:00", "all_day": false }],
  "tomorrow": [ ... ],
  "todos_overdue": [{ "title": "SQL 복습", "due": "2026-10-27", "group": "자격증" }],
  "todos_today":   [ ... ],
  "ddays": [{ "title": "네트워크관리사 필기", "date": "2026-11-01", "days_left": 4 }],
  "focus_minutes_yesterday": 95
}
```

- 반복 일정도 앱이 직접 펼쳐서 넣어요.
- **일기, 가계부, 외주, 작업 시간 추적 기록은 넣지 않아요.**
- 위치 확인과 수동 갱신은 설정 → 저장 위치에서 할 수 있어요.

## Nora에서 물려받은 기능

아래는 원작 Nora Schedule의 기능이에요. Niagara에서도 그대로 쓸 수 있어요 (TRPG 제외).

<details>
<summary>전체 기능 펼쳐 보기</summary>

#### 한눈에 보기 (홈)
- 위젯을 골라 배치하는 대시보드 (인사말, 오늘 일정, 달력, 할 일, 디데이, 뽀모도로, 작업 시간, 주간 차트, 메모, 북마크, 가계부, 칼로리 등)
- 위젯 크기 조절, 드래그로 순서 변경
- 헤더 이미지와 꾸미기용 이미지 카드 (움직이는 GIF 가능, 드래그·확대로 보이는 부분 조절)
- 달력 위젯에서 날짜에 마우스를 올리면 그날 일정 미리보기

#### 캘린더
- 월간 / 주간 / 일간 보기, 드래그로 일정 만들기
- 일정을 끌어서 다른 날로 옮기기 (주간·일간 보기에서는 15분 단위로 시간도 이동, 되돌리기 가능)
- 여러 날짜에 걸친 일정은 끊기지 않는 막대로 표시
- 반복 일정 (매일·매주·매월·매년, 횟수/종료일 지정, "이 일정만 / 모든 일정" 수정)
- 일정 완료 표시 (취소선으로 표시, 되돌리기 가능)
- 태그 (색·카테고리 지정, 태그별 필터, 첫 번째 태그 색으로 일정 표시)
- 사전 알림 (없음, 5분, 10분, 30분, 1시간, 사용자 지정) — 알림음과 시스템 알림
- 일정에 마우스를 올리면 시간·장소·태그·메모 표시
- 마감일이 있는 할 일도 캘린더에 함께 표시 (클릭해서 완료)
- 복사·붙여넣기 (Ctrl+C / Ctrl+V, Mac은 ⌘), 우클릭 메뉴, 실행 취소
- 종일 일정을 시간 일정 뒤에 보여 주는 옵션, 시간 선택 방식 고르기 (15분 단위 목록 / 오전·오후 + 시·분)

#### 할 일
- 그룹별 관리, 하위 할 일, 마감일·마감 시간

#### 디데이
- 다가오는 날 / 지난 날 카운트, 커버 사진 (움직이는 GIF 가능)
- 카드 모양 (기본 / 가로형 / 세로형)
- 매년 반복 (생일·기념일), 1일부터 세기 (사귄 날 = D+1)

#### 식단·운동
- 먹은 것과 먹을 것(예정)을 아침·점심·저녁·간식별로 기록, 체크 한 번으로 '먹었어요' 처리
- 하루 목표 칼로리 대비 남은 칼로리 (목표 − 먹은 것 + 운동 소모), 먹을 것까지 다 먹었을 때 남거나 넘치는 양
- 운동 12종류와 운동 시간 기록, 체중을 바탕으로 소모 칼로리 자동 추정 (체중 × 운동 강도(MET) × 시간, 직접 입력도 가능)
- 이번 주 7일 현황, 최근 먹은 음식 한 번에 다시 추가

#### 독서
- 읽는 중 / 읽고 싶은 책 / 다 읽은 책
- 읽은 쪽수와 진행률, 시작한 날·다 읽은 날, 별점, 독후감
- 책장 보기와 목록 보기, 검색과 정렬, 올해 읽은 책 수

#### 메모
- 리치 텍스트 편집, 자동 저장
- 그룹으로 정리 (끌어다 놓아 옮기기)

#### 그 밖의 기능
- **뽀모도로** 타이머와 집중 기록
- **작업 시간 추적**: 지정한 앱에서 보낸 시간만 기록, 자리 비움 감지, 앱·창 제목별 통계
- **통계**: 집중 시간, 할 일 완료, 작업 시간 차트
- **북마크** (폴더, 끌어서 순서 바꾸기), **가계부**
- **오버레이 패널**: 바탕화면 위에 떠 있는 작은 위젯 패널 (모서리에 붙이기, 다른 창 위에 고정, 접기·펼치기, 로그인할 때 자동 실행)

#### 꾸미기와 설정
- 라이트 / 다크 / 시스템 테마, 색상 테마 프리셋과 직접 만들기 (메인 색, 보조 색, 배경 색감)
- Windows 제목 표시줄 색이 테마를 따라감
- 스티커: 내 이미지(투명 PNG, 움직이는 GIF 포함)를 아무 페이지에나 붙이고 옮기기·크기·회전 조절 (꾸미기 모드에서만 편집, 평소에는 클릭을 방해하지 않음)
- 한국어 / English / 日本語, Pretendard 글꼴 (Niagara는 가볍게 하려고 일본어 글꼴을 넣지 않고 시스템 글꼴을 써요)
- 사이드바 페이지 순서 바꾸기·숨기기
- 화면 배율 조절
- 창 크기·위치 기억, 크기 프리셋
- 전체 데이터 백업·복원 (이미지, 식단·운동, 독서 기록 포함), 초기화

</details>

## Nora에서 옮겨 오기

Niagara는 Nora와 데이터 폴더가 달라서, 설치만 해서는 기존 데이터가 보이지 않아요.

1. Nora에서 **설정 → 내 데이터 → 백업 내보내기**로 `.nora` 파일을 만들어요.
2. Niagara에서 **설정 → 내 데이터 → 백업 가져오기**로 그 파일을 열어요.

Nora와 Niagara는 따로 저장되니까 둘을 같이 설치해 둬도 서로 영향이 없어요.

## 기술 스택

| 영역 | 사용 기술 |
|---|---|
| 앱 프레임워크 | [Tauri 2](https://tauri.app) (Windows: WebView2, macOS: WKWebView) |
| 백엔드 | Rust (edition 2024), SQLite ([rusqlite](https://github.com/rusqlite/rusqlite), 번들 빌드), chrono, serde, image, zip |
| 프론트엔드 | [Svelte 5](https://svelte.dev) (runes), TypeScript, Vite |
| 에디터 | [Tiptap 3](https://tiptap.dev) (Markdown 지원), marked, DOMPurify |
| UI | Lucide 아이콘, Pretendard 글꼴, 직접 만든 CSS 디자인 시스템 |
| Tauri 플러그인 | autostart, dialog, notification, opener, single-instance, window-state |
| 작업 시간 추적 | active-win-pos-rs (현재 활성 창 읽기) |

데이터는 OS의 앱 데이터 폴더(Windows: `%APPDATA%\com.niagara.scheduler`, macOS: `~/Library/Application Support/com.niagara.scheduler`)에 SQLite 파일과 이미지 폴더로 저장돼요.

## 설치하기

`main`에 변경이 올라오면 GitHub Actions가 Windows 설치 파일을 만들어요.

1. 이 레포의 **Actions** 탭 → **Build Windows installer** → 가장 최근의 성공한 실행을 열어요.
2. 아래 **Artifacts**에서 `niagara-scheduler-windows`를 내려받아 압축을 풀고 `…-setup.exe`를 실행해요. (GitHub 로그인이 필요해요.)
3. "Windows의 PC 보호" 창이 뜨면 **추가 정보 → 실행**을 눌러요. 서명되지 않은 개인 빌드라서 그래요.

## 직접 빌드하기

필요한 것:
- Node.js 22.12 이상 (`.nvmrc` 참고)
- Rust (rustup)
- Windows: Microsoft C++ Build Tools, WebView2 (Windows 11에는 기본 포함)
- macOS: Xcode Command Line Tools (`xcode-select --install`)

```sh
npm install
npm run tauri dev      # 개발 모드로 실행
npm run tauri build    # 설치 파일 만들기
```

빌드 결과물은 `src-tauri/target/release/bundle/` 아래에 생겨요 (Windows: `nsis/…-setup.exe`, `msi/….msi` · macOS: `macos/Niagara Scheduler.app`, `dmg/….dmg`).

그 밖의 명령:

```sh
npm run check                  # 프론트엔드 타입 검사
cd src-tauri && cargo test     # 백엔드 테스트
```

개발 중에 실제 데이터 대신 다른 폴더를 쓰려면 `NORA_DATA_DIR` 환경 변수로 지정하세요.

## 직접 빌드하기

필요한 것:
- Node.js 22.12 이상 (`.nvmrc` 참고)
- Rust (rustup)
- Windows: Microsoft C++ Build Tools, WebView2 (Windows 11에는 기본 포함)
- macOS: Xcode Command Line Tools (`xcode-select --install`)

```sh
npm install
npm run tauri dev      # 개발 모드로 실행
npm run tauri build    # 설치 파일 만들기
```

빌드 결과물은 `src-tauri/target/release/bundle/` 아래에 생겨요 (Windows: `nsis/…-setup.exe`, `msi/….msi` · macOS: `macos/Niagara Scheduler.app`, `dmg/….dmg`).

```sh
npm run check                  # 프론트엔드 타입 검사
cd src-tauri && cargo test     # 백엔드 테스트
```

개발 중에 실제 데이터 대신 다른 폴더를 쓰려면 `NORA_DATA_DIR` 환경 변수로 지정하세요. (원작과 호환되도록 이름을 그대로 뒀어요.)

## 라이선스

[MIT](LICENSE). 원작의 저작권 표시(Imgyeong Lee)를 그대로 유지하고, Niagara에서 바꾼 부분의 저작권 줄을 더했어요.

## 출처와 감사

Niagara Scheduler는 Imgyeong Lee님의 [Nora Schedule](https://github.com/ImgyeongLee/NoraSchedule)에서 출발했어요.
잘 만들어진 앱을 MIT 라이선스로 공개해 주신 덕분에 제 생활에 맞게 고쳐 쓸 수 있었어요. 감사합니다.

- 앱의 설계, 디자인, 대부분의 코드는 원작자의 작업이에요. Niagara가 바꾸거나 더한 부분은 [docs/CHANGES.md](docs/CHANGES.md)에 정리되어 있어요.
- Niagara는 원작과 별개의 프로젝트예요. 원작자가 관리하거나 지원하지 않으니, Niagara의 문제는 원작 레포가 아니라 이 레포에 남겨 주세요.
- 원작이 마음에 드신다면 [Nora Schedule](https://github.com/ImgyeongLee/NoraSchedule)도 꼭 들러 주세요.
