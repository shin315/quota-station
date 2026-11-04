# Quota Station

[English](README.md) · **Tiếng Việt**

Ứng dụng trên thanh menu macOS và khay hệ thống Windows cho biết hạn mức
Claude Code, Codex và Antigravity của bạn đã dùng bao nhiêu, chuyển qua lại
giữa các tài khoản, và giữ gọn MCP server, skill cùng file hướng dẫn của các
agent. Giao diện có tiếng Việt và tiếng Anh.

<p align="center">
  <img src="docs/screenshots/vi/popover.png" width="360" alt="Popover ở giao diện tối: mỗi dịch vụ Claude, Antigravity và Codex một phần, mở ra ở tài khoản đang dùng với phần trăm đã dùng và thời gian đến lúc đặt lại, các tài khoản khác gập dưới một dòng, và Tự động chuyển Claude ở chân">
</p>

<p align="center">
  <img src="docs/screenshots/menubar.png" width="600" alt="Mục trên thanh menu, ở thanh menu sáng và tối: đồng hồ C04 với tài khoản và phần trăm đã dùng cho Claude, Gemini và Codex qua ChatGPT, Codex qua nhà cung cấp riêng không có số, vòng xoay đang tạm dừng với hai vạch trong đồng hồ, và vòng xoay cạn với hình tam giác cảnh báo">
</p>

Quota Station tự đọc hạn mức ngay trên máy tính của bạn; không cần chương
trình nào khác để có số.

Mọi hình trong trang này dùng dữ liệu thử bịa ra, không bao giờ là tài khoản
thật.

> **Tình trạng.** Dành cho macOS 13 trở lên và Windows 10 hoặc 11 (x64). Tải
> về từ [Releases](https://github.com/shin315/quota-station/releases/latest);
> phần [Cài đặt](#cài-đặt) nói cách cài.

## Mục lục

- [Ứng dụng làm gì](#ứng-dụng-làm-gì)
- [Riêng tư và an toàn](#riêng-tư-và-an-toàn)
- [Yêu cầu](#yêu-cầu)
- [Cài đặt](#cài-đặt)
- [Cập nhật](#cập-nhật)
- [Gỡ cài đặt](#gỡ-cài-đặt)
- [Tài liệu](#tài-liệu)
- [Điều khoản dịch vụ](#điều-khoản-dịch-vụ)

## Ứng dụng làm gì

### Thanh menu và popover

Thanh menu hiện một đồng hồ nhỏ cùng tài khoản đang dùng và phần trăm đã dùng,
ví dụ `lan.acme1 74%`. Bạn chọn dịch vụ mà nó theo (Claude, Gemini hoặc Codex)
và cửa sổ hạn mức nó hiện (5 giờ, tuần, Fable tuần, hoặc cửa sổ dùng nhiều
nhất). Nhấp trái mở popover: mỗi dịch vụ một phần, mở ra ở tài khoản đang dùng
với mọi cửa sổ hạn mức và giờ đặt lại, các tài khoản khác chỉ cách một lần
nhấp. Thanh chuyển vàng từ 70 % và đỏ từ 90 %. Nhấp phải mở một menu ngắn.

### Cửa sổ chính

<img src="docs/screenshots/vi/overview.png" width="760" alt="Tổng quan của cửa sổ chính: tài khoản Claude, Antigravity và Codex trong các bảng, với phần trăm đã dùng của từng cửa sổ và nút Chuyển trên mỗi tài khoản không dùng">

`Tổng quan` gom mọi tài khoản của ba dịch vụ vào một chỗ, với `Chuyển` trên mỗi
tài khoản chưa dùng. Thanh bên chia phần còn lại: `Thống kê`, tài khoản
`Claude`, `Antigravity` và `Codex`, `Lịch sử`, các công cụ cho agent (`MCP`,
`Skills`, `Plugin`, `File hướng dẫn`, `Bộ cấu hình`, `Cập nhật`), `Sao lưu` và
`Cài đặt`. Cái chuông ở góc trên bên phải mỗi trang giữ các thông báo.

### Tài khoản Claude

<img src="docs/screenshots/vi/claude.png" width="760" alt="Tab Claude: bốn tài khoản với 5 giờ, Tuần và Fable tuần, nút Chuyển và menu ⋯ trên mỗi dòng, bên dưới là Tự động chuyển Claude với các lựa chọn tạm dừng và Chạy nền">

Quota Station tự giữ nhiều tài khoản Claude và tự chuyển Claude Code giữa
chúng: thêm tài khoản bằng cách đăng nhập trong trình duyệt, lấy tài khoản
Claude Code đang đăng nhập, hoặc dán setup-token. Trước khi chuyển, app cho biết
bao nhiêu phiên Claude Code đang chạy sẽ phải dựng lại prompt cache, rồi kiểm
tra lại là đã chuyển xong.

Vòng xoay tự động giữa các tài khoản Team, có tạm dừng, là một trong
[các công cụ nâng cao](#nâng-cao).

### Codex

<img src="docs/screenshots/vi/codex.png" width="760" alt="Tab Codex: bốn tài khoản ChatGPT và ba nhà cung cấp Custom API với nút Chuyển, ô plugin, dòng khoá API, Tự đánh thức ChatGPT và Codex trên máy này">

Giữ nhiều tài khoản ChatGPT cho Codex, cùng các nhà cung cấp Custom API trong
`config.toml` của nó, và chuyển qua lại; đăng nhập ChatGPT vẫn ở nguyên chỗ nên
plugin của Codex App vẫn chạy khi đi qua Custom API. Mỗi lần chuyển đều cho xem
trước những gì thay đổi, ghi đúng một file kèm bản lưu, và không bao giờ gửi
đăng nhập của bạn cho nhà cung cấp. `Tự đánh thức ChatGPT` có thể khởi động sớm
cửa sổ 5 giờ của một tài khoản theo lịch bạn đặt.

### Antigravity

Dòng Antigravity hiện hạn mức Gemini và Claude/GPT của tài khoản đang đăng nhập.
App còn đăng nhập được nhiều tài khoản Google và chuyển tài khoản mà
Antigravity.app và `agy` dùng: bản tải về và bản dựng từ mã nguồn bằng
`scripts/install.sh` đều có phần tài khoản Antigravity này, chỉ
`QS_ANTIGRAVITY_ACCOUNTS=0` mới bỏ phần đó ra.

### MCP server, skill, plugin và file hướng dẫn

<img src="docs/screenshots/vi/mcp.png" width="760" alt="Tab MCP: mỗi server một dòng với ô đánh dấu cho Claude Code, Codex và Antigravity, nhãn báo thiếu khoá hoặc thiếu lệnh, và các dòng cho server đã cất, server Codex tự có và bản lưu">

Một bảng cho mọi MCP server của Claude Code, Codex và Antigravity, mỗi agent một
ô đánh dấu. Các thay đổi được gom lại, cho xem trước một lần, ghi một lần kèm
bản lưu, và hoàn tác được. Khoá nằm trong Keychain và không bao giờ hiện ra.

<img src="docs/screenshots/vi/skills.png" width="760" alt="Tab Skills: mỗi skill một dòng với ô đánh dấu cho từng agent, hai skill có bản mới, Cập nhật tất cả và Cài từ GitHub">

Tab `Skills` làm như vậy cho skill, cài skill từ GitHub và báo khi một skill có
bản mới. `Plugin` liệt kê những gì mỗi agent đã cài, còn `File hướng dẫn` theo
dõi CLAUDE.md, AGENTS.md và GEMINI.md.

### Bộ cấu hình

<img src="docs/screenshots/vi/sets.png" width="760" alt="Bộ cấu hình: bộ đang dùng lệch một mục, và ba bộ với Áp dụng…, Sửa và Xoá">

Một bộ bật hoặc tắt một nhóm MCP server và skill đã chọn cho cả máy trong một
lần, sau một lần xem trước, và có thể chuyển cả Codex. App báo khi máy đã lệch
khỏi bộ, và hoàn tác được.

### Sao lưu và khôi phục

<img src="docs/screenshots/vi/backup.png" width="760" alt="Tab Sao lưu: gói mới nhất, công tắc tự sao lưu, các khoá đã lưu, và các gói của ba máy Mac">

Sao lưu MCP server, skill, plugin và file hướng dẫn lên iCloud Drive (hoặc thư
mục bạn chọn) rồi khôi phục trên bất kỳ máy Mac nào của bạn, từng mục một. Gói
sao lưu không bao giờ chứa khoá: lúc khôi phục app hỏi lại.

### Thống kê

<img src="docs/screenshots/vi/stats.png" width="760" alt="Thống kê 7 ngày: 45,5 triệu token, giá API tương đương, mỗi ngày một cột chồng cho Claude Code và Codex, cùng số token theo tài khoản Claude, theo dự án và theo model">

Số token Claude Code và Codex đã dùng trên máy này, đọc từ log phiên của chính
chúng (chỉ đọc con số, không bao giờ đọc nội dung hội thoại): theo ngày hoặc
giờ, theo tài khoản Claude, dự án và model, kèm giá nếu tính theo giá API.

### Cập nhật Agent

<img src="docs/screenshots/vi/updates.png" width="760" alt="Cập nhật Agent: Claude Code đã là bản mới nhất, Codex CLI và Antigravity CLI có bản mới với nút Cập nhật, và Tự động kiểm tra">

Kiểm tra Claude Code, Codex CLI và Antigravity CLI đã là bản mới nhất chưa, và
chỉ chạy lệnh cập nhật của chính công cụ đó khi bạn bấm `Cập nhật`.

### Thông báo

<img src="docs/screenshots/vi/notifications.png" width="760" alt="Bảng thông báo mở dưới cái chuông trên Tổng quan: hai thông báo hôm nay, Antigravity đổi về tài khoản cũ và Team A rời vòng lúc 14:45, mỗi thông báo kèm các nút của thẻ macOS">

Thông báo khi hạn mức Claude sắp cạn, khi một tài khoản cần đăng nhập
lại, khi lần chuyển Antigravity bị ghi đè, khi skill hay công cụ có bản mới, và
khi một biến môi trường (như `ANTHROPIC_API_KEY` trong `~/.zshrc`) đè đăng nhập
của agent. Mỗi thông báo đến một lần, dưới dạng thẻ macOS có nút bấm, và nằm lại
dưới cái chuông.

### Cài đặt, chủ đề và ngôn ngữ

<img src="docs/screenshots/vi/settings.png" width="760" alt="Cài đặt › Hiển thị: Giao diện, sáu chủ đề vẽ như cửa sổ thu nhỏ, Hiện email, và dịch vụ cùng con số trên thanh menu">

Cài đặt có `Chung` (ngôn ngữ, mở khi đăng nhập, thư mục sao lưu, cập nhật
app), `Hiển thị` (sáng hoặc tối, sáu chủ đề, thanh menu), `Thông báo` và
`Dịch vụ`. Tắt `Hiện email` thì tài khoản được gọi theo vị trí hoặc số thứ tự,
tiện để chụp màn hình.

Cài đặt còn có `Nâng cao`, nơi hiện ra [các công cụ nâng cao](#nâng-cao).

### Lần chạy đầu

<img src="docs/screenshots/vi/first-run.png" width="760" alt="Bước thứ hai của lần chạy đầu, Máy này có gì: đã thấy Claude Code, Codex và Antigravity">

Bản cài mới mở ra một phần thiết lập ngắn: ngôn ngữ, máy này có gì, tài khoản
Claude, quyền và thanh menu, xong.

### Thêm nữa

- **Lịch sử** mọi lần chuyển tài khoản trên máy này.
- **Cập nhật app**: app hỏi GitHub Releases xem có bản mới không và báo cho bạn;
  app không bao giờ tự cài.

### Nâng cao

`Cài đặt › Nâng cao › Hiện phần nâng cao`, tắt ở bản cài mới, thêm vòng xoay
Claude tự động cùng tạm dừng và trình sửa luật, gửi luật sang các máy Mac khác
qua SSH, `Chép sang Mac khác`, và cập nhật từ bản clone mã nguồn.

Mọi phần được mô tả đầy đủ trong [docs/features.md](docs/features.md) (tiếng
Anh, kèm nhãn tiếng Việt).

## Riêng tư và an toàn

- Con số được đọc ngay trên máy bạn, từ endpoint hạn mức của chính từng dịch vụ
  bằng đăng nhập của chính các agent, và không gửi đi đâu khác. Không tài khoản,
  không telemetry, không máy chủ riêng của app.
- Không gì được ghi vào file của agent nếu chưa có bước xem trước hoặc xác nhận,
  mỗi lần ghi đều giữ bản lưu, và một file đã đổi sau lúc xem trước sẽ chặn lần
  ghi.
- Khoá và đăng nhập mà app giữ nằm trong Keychain của macOS (các tài khoản
  ChatGPT phụ nằm trong thư mục chỉ bạn mở được). Khoá không bao giờ hiện trong
  cửa sổ, log hay gói sao lưu.
  Khoá chỉ sang máy Mac khác khi bạn đánh dấu `Gửi cả khoá qua SSH`.

[SECURITY.md](SECURITY.md) có chi tiết, và
[docs/how-it-works.md](docs/how-it-works.md) liệt kê mọi thứ app đọc, chạy và
ghi (tiếng Anh).

## Yêu cầu

- macOS 13 trở lên (Apple Silicon hoặc Intel), hoặc Windows 10 hay 11 trên x64.
- Để dựng từ mã nguồn: Rust stable (1.85 trở lên) và Node.js 20 trở lên. Bản
  tải về không cần cả hai, và máy Mac không có chúng vẫn cài được bản dựng trên
  một máy Mac khác.
- Để có số, không cần gì thêm: Codex, `agy` hoặc Antigravity.app, và Claude Code
  chỉ cần đã đăng nhập.
- Để đăng nhập tài khoản Claude mới, và cho `quota-station claude run`: lệnh
  `claude` của Claude Code.
- Để đăng nhập tài khoản Antigravity (bản tải về và bản dựng bằng
  `scripts/install.sh` đều có; `QS_ANTIGRAVITY_ACCOUNTS=0` bỏ phần đó ra):
  [file OAuth client](docs/how-it-works.md#the-oauth-client-file), do bạn tự
  tạo với hai giá trị lấy từ mã nguồn Cockpit Tools: trên Mac là
  `~/Library/Application Support/com.shin315.quota-station/oauth-client.json`
  (quyền 600), trên Windows là
  `%APPDATA%\com.shin315.quota-station\oauth-client.json` (hướng dẫn có lệnh
  PowerShell để tạo file).
- Cho các công cụ nâng cao cần tới máy Mac khác: một SSH alias tới từng máy,
  đăng nhập không cần hỏi (dùng khoá, `BatchMode=yes`).

## Cài đặt

Tải app từ
[bản phát hành mới nhất](https://github.com/shin315/quota-station/releases/latest):
file DMG cho macOS, bộ cài cho Windows. Dựng từ mã nguồn chỉ dành cho macOS.

### macOS

Tải `Quota.Station_<version>_universal.dmg`, mở ra rồi kéo `Quota Station` vào
`Applications`. Một file DMG chạy được trên cả Apple Silicon lẫn Intel. App đã
được ký bằng Developer ID và được Apple notarize, nên lần mở đầu macOS chỉ hỏi
câu quen thuộc về app tải từ Internet: chọn `Mở` (Open).

Cài bằng Homebrew: `brew install --cask shin315/tap/quota-station` (cập nhật
bằng `brew upgrade --cask quota-station`).

### Windows

Tải `Quota.Station_<version>_x64-setup.exe` rồi chạy. Bộ cài chưa được ký, nên
Windows sẽ cảnh báo:

1. Trình duyệt có thể báo file này ít được tải: chọn `Giữ lại` (Keep) (trong
   Edge, ở menu `…` cạnh file tải về; nó có thể hỏi thêm một lần, `Vẫn giữ lại`
   (Keep anyway)).
2. SmartScreen báo "Windows protected your PC": bấm `Thông tin thêm` (More
   info), rồi `Vẫn chạy` (Run anyway).

Bộ cài chỉ cài cho người dùng Windows của bạn và không cần quyền quản trị. Sau
đó Quota Station nằm ở khay hệ thống bên phải thanh tác vụ; lúc đầu Windows có
thể giấu nó dưới mũi tên `^`, bạn kéo nó ra thanh tác vụ được.

Để kiểm tra bộ cài đúng là bản đã phát hành, tải `SHA256SUMS.txt` của cùng bản
phát hành về cạnh nó rồi, trong PowerShell ở thư mục đó:

```powershell
$sum = (Select-String '_x64-setup.exe' .\SHA256SUMS.txt).Line.Split(' ')[0]
(Get-FileHash .\Quota.Station_<version>_x64-setup.exe).Hash -eq $sum
```

Ra `True` là khớp. Các mã này cũng nằm ở cuối ghi chú phát hành.

### Từ mã nguồn (macOS)

Từ một bản clone, trên máy Mac có Rust và Node.js:

```sh
git clone https://github.com/shin315/quota-station.git
cd quota-station
scripts/install.sh
```

Script dựng app, ký (bằng định danh `Apple Development` của bạn, hoặc ad-hoc),
chép vào `/Applications/Quota Station.app` và mở lên. Script không bao giờ thêm
mục đăng nhập và không bao giờ đòi `sudo`. Nó dựng kèm phần tài khoản
Antigravity; `QS_ANTIGRAVITY_ACCOUNTS=0` bỏ phần đó ra. `scripts/install.sh
--help` liệt kê các tuỳ chọn.

Trên máy không dựng được, hãy tạo gói trên một máy dựng được rồi chép sang:

```sh
scripts/install.sh --package ~/Desktop          # ghi ra quota-station-<version>.zip
# trên máy kia, nơi file zip được chép tới:
ditto -x -k quota-station-<version>.zip . && ./quota-station-<version>/install.sh --app "./quota-station-<version>/Quota Station.app"
```

### Sau khi cài

Trên máy Mac hoặc PC đăng nhập tài khoản Antigravity, đặt
[file OAuth client](docs/how-it-works.md#the-oauth-client-file) vào chỗ, và bật
`Cài đặt › Chung › Mở khi đăng nhập` nếu muốn app chạy cùng máy.

Đang chuyển từ claude-swap? Phần
[cutover](docs/claude-accounts.md#cutover) nói cách làm.

## Cập nhật

Bản tải về hỏi GitHub Releases mỗi ngày một lần, khi `Tự kiểm tra bản mới của
app` trong `Cài đặt › Chung › Cập nhật` đang bật, và báo khi có bản mới: một
dải thông báo trong `Tổng quan` và một dòng trong popover. `Tải và cài` mở file
DMG hoặc bộ cài mới trong trình duyệt; cài như lần đầu, đè lên bản cũ (trên Mac
thì thoát app trước). App không bao giờ tự tải hay tự cài gì.

Bản dựng từ mã nguồn cũng được báo khi có bản mới, nhưng cập nhật từ bản clone
của nó. Từ Terminal, trong thư mục clone:

```sh
git pull --ff-only && scripts/install.sh
```

`Cài đặt › Nâng cao` cũng làm được việc này khi bật `Hiện phần nâng cao`: đặt
`Mã nguồn` là thư mục clone rồi bấm `Cập nhật từ mã nguồn`. App kéo mã mới, dựng
và cài khi bản clone có commit mới, rồi đề nghị `Mở lại`; bản clone có commit
riêng không bao giờ bị merge hay reset, và lần dựng quá 30 phút thì dừng.

## Gỡ cài đặt

1. Trong `Cài đặt › Chung`, tắt `Mở khi đăng nhập` (hoặc bỏ Quota Station trong
   System Settings › General › Login Items trên Mac, hay trong `Ứng dụng khởi
   động` (Startup apps) của Task Manager trên Windows).
   Nếu app đang chạy vòng xoay Claude, tắt nó trong tab `Claude` (hoặc chạy
   `quota-station claude rotation off`).
   Muốn bộ tự chuyển cũ xoay vòng Claude trở lại thì nạp lại plist của nó
   ([cutover](docs/claude-accounts.md#cutover)).
2. Thoát Quota Station từ menu của nó. Trên Mac, kéo
   `/Applications/Quota Station.app` vào Thùng rác. Trên Windows, mở Cài đặt ›
   Ứng dụng › Ứng dụng đã cài đặt (Settings › Apps › Installed apps; trên
   Windows 10 là Ứng dụng và tính năng), rồi `…` cạnh Quota Station › `Gỡ cài
   đặt` (Uninstall).
3. Tuỳ chọn, xoá dữ liệu của app. Trên Mac:
   `~/Library/Application Support/com.shin315.quota-station/` (cài đặt, thư mục
   tài khoản Claude, chỉ mục và số đọc Antigravity, lịch sử, thông báo và
   `oauth-client.json`), và các mục Keychain riêng của app, service
   `com.shin315.quota-station.claude` và
   `com.shin315.quota-station.antigravity`, trong Keychain Access. Xoá các tài
   khoản Claude trong app (trừ tài khoản đang dùng) để xoá mục của chúng; Claude
   Code vẫn giữ đăng nhập của nó. Đăng xuất các tài khoản Antigravity trong app
   trước để xoá các mục đó. Đừng đụng vào mục `gemini`/`antigravity`:
   Antigravity.app và agy dùng nó.
   Trên Windows: `%APPDATA%\com.shin315.quota-station\` (cài đặt, tài khoản,
   số đọc, lịch sử, thông báo và `oauth-client.json`) và
   `%LOCALAPPDATA%\com.shin315.quota-station\` (những gì chỉ ở lại máy này,
   gồm cả khoá riêng của app, được Windows niêm phong); xoá cả hai thư mục là
   xoá hết. Claude Code vẫn giữ đăng nhập của nó. Đừng đụng vào thông tin đăng
   nhập `gemini:antigravity` trong Credential Manager: Antigravity và agy dùng
   nó.

File luật xoay vòng của bộ tự chuyển cũ được dùng chung với bộ tự chuyển đó,
nên vẫn ở lại.

## Tài liệu

Các trang trong `docs/` viết bằng tiếng Anh.

| Trang | Nội dung |
|---|---|
| [docs/features.md](docs/features.md) | Mọi phần của app, đầy đủ |
| [docs/how-it-works.md](docs/how-it-works.md) | Các bộ đọc hạn mức, tài khoản Claude và Antigravity, dòng lệnh, những gì app đọc, chạy và ghi, file OAuth client |
| [docs/development.md](docs/development.md) | Dựng app, các feature khi dựng, bản xem trước trên trình duyệt, kiểm thử, cấu trúc mã |
| [docs/releasing.md](docs/releasing.md) | Workflow phát hành và phần thiết lập một lần nó cần |
| [docs/claude-accounts.md](docs/claude-accounts.md) | Thiết kế phần tài khoản Claude |
| [docs/themes.md](docs/themes.md) | Sáu chủ đề |
| [docs/ui-guidelines.md](docs/ui-guidelines.md) | Cách viết chữ trên giao diện, tiếng Việt và tiếng Anh |
| [SECURITY.md](SECURITY.md) | Khoá, token, những gì không bao giờ đọc, và cách báo lỗi bảo mật |

## Điều khoản dịch vụ

Đăng nhập tài khoản Antigravity bằng OAuth client của chính Antigravity, và đọc
hạn mức theo cách Antigravity đọc, mang cùng rủi ro về điều khoản dịch vụ như
khi dùng Cockpit Tools, không hơn: app gọi đúng những lệnh đó, cùng user agent
và header, không dày hơn. `Tự đánh thức ChatGPT` gửi yêu cầu theo lịch, điều mà
điều khoản của OpenAI không nói tới: chỉ dùng với tài khoản của chính bạn.
[SECURITY.md](SECURITY.md) nói điều gì có thể xảy ra và thay đổi nào sẽ làm rủi
ro lớn hơn.

---

<sub>MIT, trừ phần tài khoản Antigravity tuỳ chọn (dựa một phần trên Cockpit Tools, CC BY-NC-SA 4.0). Chi tiết trong [LICENSE](LICENSE), [LICENSES/](LICENSES), [NOTICE](NOTICE) và [THIRD_PARTY.md](THIRD_PARTY.md).</sub>
