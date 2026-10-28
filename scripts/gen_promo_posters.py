import os
import subprocess

OUT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "docs", "promo"))
os.makedirs(OUT_DIR, exist_ok=True)

COMMON_FONTS_AND_RESET = """
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  width: 1200px;
  height: 1600px;
  overflow: hidden;
  background: radial-gradient(circle at 50% 48%, #FAF6F0 0%, #F7F7F4 55%, #F3F3EF 100%);
  font-family: "Microsoft YaHei UI", "PingFang SC", "Hiragino Sans GB", "Segoe UI", system-ui, sans-serif;
  color: #141413;
  -webkit-font-smoothing: antialiased;
  position: relative;
}
/* Subtle warm paper grain feel */
.paper-glow {
  position: absolute;
  left: 50%;
  top: 52%;
  transform: translate(-50%, -50%);
  width: 820px;
  height: 760px;
  background: radial-gradient(circle, rgba(255, 168, 76, 0.08) 0%, rgba(34, 197, 94, 0.04) 45%, rgba(0,0,0,0) 72%);
  pointer-events: none;
  z-index: 0;
}
"""

# Reusable UI mockup HTML (Card + Floating Rail)
def render_mockup_html(scale=1.0, show_detailed_extras=False):
    extra_rows = ""
    if show_detailed_extras:
        extra_rows = """
        <div style="display:flex; justify-content:space-between; align-items:center; margin-top:10px; padding-top:10px; border-top:1px solid rgba(255,255,255,0.08); font-size:15px;">
          <span style="color:#9CA3AF;">额度重置券 (app-server)</span>
          <span style="color:#F3F4F6; font-family:'Segoe UI', monospace; font-weight:600;">2 张可用 · 11/05 到期</span>
        </div>
        <div style="display:flex; justify-content:space-between; align-items:center; margin-top:7px; font-size:15px;">
          <span style="color:#9CA3AF;">提示缓存 (30 分钟)</span>
          <span style="color:#10B981; font-family:'Segoe UI', monospace; font-weight:600;">至少剩 24 分钟</span>
        </div>
        """

    return f"""
    <div style="position:relative; z-index:2; display:flex; align-items:center; justify-content:center; gap:26px; transform:scale({scale}); transform-origin:center center;">
      <!-- Dark Floating Detail Card -->
      <div style="position:relative; width:560px; background:linear-gradient(180deg, #121316 0%, #0B0C0E 100%); border-radius:34px; padding:36px 38px 32px 38px; color:#F5F5F7; box-shadow: 0 32px 70px rgba(0,0,0,0.28), 0 8px 24px rgba(0,0,0,0.16), inset 0 1px 0 rgba(255,255,255,0.12); border:1px solid rgba(255,255,255,0.08);">
        <!-- Right-pointing Bubble Arrow -->
        <div style="position:absolute; right:-15px; top:126px; width:30px; height:30px; background:#111215; transform:rotate(45deg); border-top:1px solid rgba(255,255,255,0.10); border-right:1px solid rgba(255,255,255,0.10);"></div>

        <!-- Card Header -->
        <div style="display:flex; align-items:center; justify-content:space-between;">
          <div style="display:flex; align-items:center; gap:13px;">
            <!-- OpenAI Blossom SVG -->
            <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 2a4.5 4.5 0 0 0-3.89 2.25L4.5 10.5a4.5 4.5 0 0 0 0 4.5l3.61 6.25a4.5 4.5 0 0 0 7.78 0l3.61-6.25a4.5 4.5 0 0 0 0-4.5L15.89 4.25A4.5 4.5 0 0 0 12 2Z" stroke="none"/>
              <circle cx="12" cy="12" r="3.2"/>
              <path d="M12 2.5c2.5 0 4.5 2 4.5 4.5v2.2M16.1 4.9c2.2 1.2 2.9 4 1.7 6.2l-1.1 1.9M20.2 12c1.2 2.2.5 5-1.7 6.2l-1.9 1.1M16.5 19.5c0 2.5-2 4.5-4.5 4.5s-4.5-2-4.5-4.5v-2.2M7.9 19.1c-2.2-1.2-2.9-4-1.7-6.2l1.1-1.9M3.8 12c-1.2-2.2-.5-5 1.7-6.2l1.9-1.1"/>
            </svg>
            <span style="font-size:29px; font-weight:700; letter-spacing:0.2px; color:#FFFFFF;">Codex 用量</span>
          </div>
          <span style="font-size:15px; font-weight:600; color:#9CA3AF; background:rgba(255,255,255,0.07); padding:4px 12px; border-radius:999px; border:1px solid rgba(255,255,255,0.08);">Pro · 刚刚更新</span>
        </div>

        <!-- Quota Window Section -->
        <div style="margin-top:26px;">
          <div style="display:flex; justify-content:space-between; align-items:baseline;">
            <span style="font-size:22px; font-weight:700; color:#F3F4F6;">每周限额</span>
            <span style="font-size:16px; color:#9CA3AF; font-family:'Segoe UI', sans-serif;">估算价值 $220 · 已用 $55</span>
          </div>
          <!-- Progress Bar -->
          <div style="width:100%; height:14px; background:#262930; border-radius:999px; margin:14px 0 13px 0; overflow:hidden;">
            <div style="width:27%; height:100%; background:linear-gradient(90deg, #10B981 0%, #22C55E 100%); border-radius:999px; box-shadow: 0 0 12px rgba(34,197,94,0.45);"></div>
          </div>
          <div style="display:flex; justify-content:space-between; align-items:center; font-size:21px;">
            <span style="font-weight:700; color:#F3F4F6;">已用 25%</span>
            <span style="font-weight:600; color:#8E929B;">9月26日 16:27 重置</span>
          </div>
        </div>

        {extra_rows}

        <!-- Divider -->
        <div style="height:1px; background:rgba(255,255,255,0.11); margin:24px 0 22px 0;"></div>

        <!-- Reset Card Announcement Section -->
        <div>
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <div style="display:flex; align-items:center; gap:11px;">
              <!-- Ticket Icon -->
              <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#F59E0B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="2" y="6" width="20" height="12" rx="2"/>
                <path d="M6 12h.01M18 12h.01M10 10h4v4h-4z"/>
              </svg>
              <span style="font-size:25px; font-weight:700; color:#FFFFFF;">重置卡</span>
            </div>
            <span style="font-size:20px; font-weight:700; color:#F59E0B;">已预告</span>
          </div>

          <div style="margin-top:16px; font-size:26px; font-weight:700; color:#FFFFFF; letter-spacing:0.2px;">
            Tibo 预告发放重置卡
          </div>

          <div style="margin-top:8px; font-size:23px; font-weight:700; color:#F59E0B;">
            预计 22 小时后
          </div>

          <div style="margin-top:14px; font-size:20px; color:#D1D5DB; line-height:1.45; font-weight:500;">
            好吧。但它也仍然会在周二到来。
          </div>

          <!-- Footer Links -->
          <div style="margin-top:18px; display:flex; align-items:center; gap:9px; font-size:19px; color:#9CA3AF; font-weight:600;">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#9CA3AF" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>
              <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>
            </svg>
            <span>AI Hot News</span>
            <span>·</span>
            <span>原帖</span>
          </div>
        </div>
      </div>

      <!-- Right Floating Edge Rail (3 Rings) -->
      <div style="width:128px; background:linear-gradient(180deg, #141519 0%, #0A0B0D 100%); border-radius:30px; padding:22px 14px; display:flex; flex-direction:column; align-items:center; gap:22px; box-shadow: 0 28px 60px rgba(0,0,0,0.30), inset 0 1px 0 rgba(255,255,255,0.12); border:1px solid rgba(255,255,255,0.09); transform: perspective(600px) rotateY(-8deg);">
        <!-- Ring 1: Codex 25% + Orange Dot -->
        <div style="display:flex; flex-direction:column; align-items:center;">
          <div style="position:relative; width:82px; height:82px; display:flex; align-items:center; justify-content:center;">
            <svg width="82" height="82" viewBox="0 0 44 44" style="transform:rotate(-90deg);">
              <circle cx="22" cy="22" r="20.5" fill="none" stroke="rgba(255,255,255,0.14)" stroke-width="1.3"/>
              <circle cx="22" cy="22" r="20.5" fill="none" stroke="#A7F3D0" stroke-width="1.5" stroke-linecap="round" stroke-dasharray="32 130"/>
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#262930" stroke-width="3.2"/>
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#10B981" stroke-width="3.2" stroke-linecap="round" stroke-dasharray="26 104"/>
            </svg>
            <div style="position:absolute; inset:11px; border-radius:999px; background:#181A20; display:flex; align-items:center; justify-content:center; color:#FFFFFF;">
              <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="1.8">
                <circle cx="12" cy="12" r="3"/>
                <path d="M12 3c2.2 0 4 1.8 4 4v2M15.6 5.2c1.9 1.1 2.5 3.5 1.4 5.4l-1 1.7M19 12c1.1 1.9.4 4.3-1.4 5.4l-1.7 1M16 19c0 2.2-1.8 4-4 4s-4-1.8-4-4v-2M8.4 18.8c-1.9-1.1-2.5-3.5-1.4-5.4l1-1.7M5 12c-1.1-1.9-.4-4.3 1.4-5.4l1.7-1"/>
              </svg>
            </div>
          </div>
          <div style="display:flex; align-items:center; gap:6px; margin-top:4px;">
            <span style="font-size:21px; font-weight:700; color:#F3F4F6; font-family:'Segoe UI', sans-serif;">25%</span>
            <span style="width:11px; height:11px; border-radius:999px; background:#F59E0B; box-shadow:0 0 8px rgba(245,158,11,0.8);"></span>
          </div>
        </div>

        <!-- Ring 2: ZCode / Claude 23% -->
        <div style="display:flex; flex-direction:column; align-items:center;">
          <div style="position:relative; width:82px; height:82px; display:flex; align-items:center; justify-content:center;">
            <svg width="82" height="82" viewBox="0 0 44 44" style="transform:rotate(-90deg);">
              <circle cx="22" cy="22" r="20.5" fill="none" stroke="rgba(255,255,255,0.14)" stroke-width="1.3"/>
              <circle cx="22" cy="22" r="20.5" fill="none" stroke="#A7F3D0" stroke-width="1.5" stroke-linecap="round" stroke-dasharray="28 130"/>
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#262930" stroke-width="3.2"/>
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#10B981" stroke-width="3.2" stroke-linecap="round" stroke-dasharray="24 104"/>
            </svg>
            <div style="position:absolute; inset:11px; border-radius:999px; background:#181A20; display:flex; align-items:center; justify-content:center; color:#FFFFFF;">
              <div style="width:22px; height:22px; border:2px solid #E5E7EB; border-radius:5px; display:flex; align-items:center; justify-content:center; font-size:12px; font-weight:800; font-family:monospace;">Z</div>
            </div>
          </div>
          <span style="font-size:21px; font-weight:700; color:#F3F4F6; margin-top:4px; font-family:'Segoe UI', sans-serif;">23%</span>
        </div>

        <!-- Ring 3: BotMark 81% (Red Ring + Cute White Ghost Bot Looking Left) -->
        <div style="display:flex; flex-direction:column; align-items:center;">
          <div style="position:relative; width:82px; height:82px; display:flex; align-items:center; justify-content:center;">
            <svg width="82" height="82" viewBox="0 0 44 44" style="transform:rotate(-90deg);">
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#262930" stroke-width="3.2"/>
              <circle cx="22" cy="22" r="16.5" fill="none" stroke="#EF4444" stroke-width="3.2" stroke-linecap="round" stroke-dasharray="84 104"/>
            </svg>
            <div style="position:absolute; inset:11px; border-radius:999px; background:#181A20; display:flex; align-items:center; justify-content:center;">
              <svg width="32" height="32" viewBox="0 0 36 36">
                <path d="M18 5C11.5 5 7 10 7 17.5C7 24.5 9.5 30 13 30C15 30 16 28.5 18 28.5C20 28.5 21 30 23 30C26.5 30 29 24.5 29 17.5C29 10 24.5 5 18 5Z" fill="#F9FAFB"/>
                <ellipse cx="13.5" cy="17" rx="2.1" ry="3.0" fill="#111827"/>
                <ellipse cx="19.5" cy="17" rx="2.1" ry="3.0" fill="#111827"/>
              </svg>
            </div>
          </div>
          <span style="font-size:21px; font-weight:700; color:#F3F4F6; margin-top:4px; font-family:'Segoe UI', sans-serif;">81%</span>
        </div>
      </div>
    </div>
    """

# Poster 1: Exact style match to reference Image 1 (for Windows)
POSTER_1_HTML = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<style>
{COMMON_FONTS_AND_RESET}
</style>
</head>
<body>
  <div class="paper-glow"></div>
  <div style="position:relative; z-index:1; width:100%; height:100%; display:flex; flex-direction:column; align-items:center; justify-content:space-between; padding:88px 60px 68px 60px;">
    <!-- Top Header Group -->
    <div style="text-align:center;">
      <div style="font-size:38px; font-weight:600; color:#3F3F3C; letter-spacing:1.5px; margin-bottom:26px;">
        Windows 屏幕边缘小工具
      </div>
      <div style="font-size:108px; font-weight:900; color:#111110; line-height:1.14; letter-spacing:-1px;">
        <div>Codex 额度</div>
        <div>抬眼就能看</div>
      </div>
      <div style="font-size:38px; font-weight:600; color:#575652; letter-spacing:2px; margin-top:32px;">
        用量查看 · 重置预告 · 确认提醒
      </div>
    </div>

    <!-- Center Mockup -->
    <div style="margin: 12px 0;">
      {render_mockup_html(scale=1.08, show_detailed_extras=False)}
    </div>

    <!-- Bottom Legend & Branding -->
    <div style="text-align:center; display:flex; flex-direction:column; align-items:center;">
      <div style="font-size:25px; font-weight:500; color:#6E6D68; letter-spacing:0.8px;">
        界面为历史消息示例，非当前重置通知
      </div>

      <div style="display:flex; align-items:center; gap:26px; margin-top:34px; font-size:34px; font-weight:800; color:#181816;">
        <div style="display:flex; align-items:center; gap:12px;">
          <span style="width:26px; height:26px; border-radius:999px; background:#F59E0B; display:inline-block;"></span>
          <span>预告</span>
        </div>
        <span style="color:#787772; font-weight:600;">→</span>
        <div style="display:flex; align-items:center; gap:12px;">
          <span style="width:26px; height:26px; border-radius:999px; background:#10B981; display:inline-block;"></span>
          <span>确认</span>
        </div>
      </div>

      <div style="margin-top:46px;">
        <div style="font-size:40px; font-weight:900; color:#111110; letter-spacing:0.5px;">
          Pulse for Windows
        </div>
        <div style="font-size:25px; font-weight:500; color:#6E6D68; margin-top:8px;">
          免费开源 · 原生 Windows 10/11 · CNDDVP/Pulse
        </div>
      </div>
    </div>
  </div>
</body>
</html>
"""

# Poster 2: Zoomed-in Clean Close-Up (matching reference Image 2 + Detailed Card rows)
POSTER_2_HTML = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<style>
{COMMON_FONTS_AND_RESET}
</style>
</head>
<body>
  <div class="paper-glow" style="width:980px; height:980px;"></div>
  <div style="position:relative; z-index:1; width:100%; height:100%; display:flex; flex-direction:column; align-items:center; justify-content:center; padding:60px;">
    {render_mockup_html(scale=1.34, show_detailed_extras=True)}
  </div>
</body>
</html>
"""

# Poster 3: v0.8.0 Four Flagship Features Editorial Card
POSTER_3_HTML = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<style>
{COMMON_FONTS_AND_RESET}
.feat-card {{
  background: #FFFFFF;
  border: 2px solid #E4E4DC;
  border-radius: 28px;
  padding: 34px 38px;
  box-shadow: 0 14px 34px rgba(0,0,0,0.04);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
}}
.tag-pill {{
  display: inline-block;
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 18px;
  font-weight: 800;
  font-family: "Segoe UI", monospace;
  background: #141413;
  color: #FBFBF8;
}}
.lime-pill {{
  display: inline-block;
  padding: 5px 14px;
  border-radius: 999px;
  font-size: 18px;
  font-weight: 800;
  background: #C8F03C;
  color: #141413;
  border: 1.5px solid #141413;
}}
</style>
</head>
<body>
  <div class="paper-glow"></div>
  <div style="position:relative; z-index:1; width:100%; height:100%; display:flex; flex-direction:column; justify-content:space-between; padding:82px 76px 68px 76px;">
    <!-- Header -->
    <div>
      <div style="display:flex; align-items:center; justify-content:space-between;">
        <span class="lime-pill">NEW RELEASE · v0.8.0</span>
        <span style="font-size:22px; font-weight:700; color:#6E6D68; font-family:monospace;">github.com/CNDDVP/Pulse</span>
      </div>
      <div style="font-size:74px; font-weight:900; color:#111110; line-height:1.15; margin-top:22px; letter-spacing:-1px;">
        不止看额度，更是你的<br>AI 编码全能驾驶舱
      </div>
      <div style="font-size:28px; font-weight:600; color:#575652; margin-top:16px;">
        18+ 供应商实时圆环 · 本地 Token 账本 · 零遥测全开源
      </div>
    </div>

    <!-- 2x2 Feature Grid -->
    <div style="display:grid; grid-template-columns: 1fr 1fr; gap:26px; margin: 20px 0;">
      <!-- Feature 1 -->
      <div class="feat-card">
        <div>
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <span class="tag-pill">01 · CODEX RESET</span>
            <span style="font-size:28px;">🟠 → 🟢</span>
          </div>
          <div style="font-size:32px; font-weight:800; margin-top:20px; color:#111110;">
            公开重置预告 + 官方重置券
          </div>
          <div style="font-size:22px; color:#575652; line-height:1.55; margin-top:12px;">
            圆环旁橙点预告、绿点确认，集成 AI Hot News 与 Tibo 原帖直达；支持通过本机 <code style="background:#F2F2EC;padding:2px 6px;border-radius:6px;">codex app-server</code> 读取官方重置券数量与到期时间，并自动交叉验证本账号额度是否实际回升。
          </div>
        </div>
      </div>

      <!-- Feature 2 -->
      <div class="feat-card">
        <div>
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <span class="tag-pill">02 · CODEX SIGNALS</span>
            <span style="font-size:24px; font-weight:800; font-family:monospace; color:#F59E0B;">518n−2</span>
          </div>
          <div style="font-size:32px; font-weight:800; margin-top:20px; color:#111110;">
            被动「降智迹象」本地检测
          </div>
          <div style="font-size:22px; color:#575652; line-height:1.55; margin-top:12px;">
            不发探测包、不劫持流量，纯本地只读扫描 <code style="background:#F2F2EC;padding:2px 6px;border-radius:6px;">~/.codex/sessions</code>，客观统计长推理停在 <code style="background:#F2F2EC;padding:2px 6px;border-radius:6px;">518n−2</code> 晶格截断点的比例，以及模型或推理强度被静默调低的次数。
          </div>
        </div>
      </div>

      <!-- Feature 3 -->
      <div class="feat-card">
        <div>
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <span class="tag-pill">03 · DETAILED CARD</span>
            <span style="font-size:24px; font-weight:800; font-family:monospace; color:#10B981;">$220 VAL</span>
          </div>
          <div style="font-size:32px; font-weight:800; margin-top:20px; color:#111110;">
            详细卡片 & 缓存过期倒计时
          </div>
          <div style="font-size:22px; color:#575652; line-height:1.55; margin-top:12px;">
            悬停直接推算每条限额折合的公开 API 美元/人民币价值，展示今日/7天/31天 Token 与 31 天柱状图、主力模型、缓存命中率，以及 Claude Code 与 Codex 对话的 Prompt 缓存存活倒计时。
          </div>
        </div>
      </div>

      <!-- Feature 4 -->
      <div class="feat-card">
        <div>
          <div style="display:flex; justify-content:space-between; align-items:center;">
            <span class="tag-pill">04 · RECAP & EXTENSIONS</span>
            <span style="font-size:24px; font-weight:800; font-family:monospace; color:#111110;">8 CARDS</span>
          </div>
          <div style="font-size:32px; font-weight:800; margin-top:20px; color:#111110;">
            日月年报分享卡 & 免 Fork 扩展
          </div>
          <div style="font-size:22px; color:#575652; line-height:1.55; margin-top:12px;">
            一键生成日/周/月/年 8 张印刷风总结卡片（含订阅回本倍数与项目名一键打码）；支持放入 <code style="background:#F2F2EC;padding:2px 6px;border-radius:6px;">.ps1/.js/.py</code> 脚本免 Fork 接入任意中转站圆环、余额与 TTFT 延迟趋势线。
          </div>
        </div>
      </div>
    </div>

    <!-- Footer -->
    <div style="display:flex; align-items:center; justify-content:space-between; border-top:2px solid #E4E4DC; padding-top:28px;">
      <div>
        <div style="font-size:34px; font-weight:900; color:#111110;">Pulse for Windows v0.8.0</div>
        <div style="font-size:22px; color:#6E6D68; margin-top:4px;">支持安装版 (.exe) 与解压即用便携版 (.zip) · 局域网多设备同步 · StepFun 扫码无感续期</div>
      </div>
      <div style="padding:14px 26px; background:#141413; color:#FBFBF8; border-radius:16px; font-size:22px; font-weight:700; font-family:monospace;">
        ★ Star on GitHub
      </div>
    </div>
  </div>
</body>
</html>
"""

posters = [
    ("promo-1-cover", POSTER_1_HTML),
    ("promo-2-closeup", POSTER_2_HTML),
    ("promo-3-features", POSTER_3_HTML),
]

chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
if not os.path.exists(chrome_path):
    chrome_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"

for name, html in posters:
    html_path = os.path.join(OUT_DIR, f"{name}.html")
    png_path = os.path.join(OUT_DIR, f"{name}.png")
    with open(html_path, "w", encoding="utf-8") as f:
        f.write(html)
    cmd = [
        chrome_path,
        "--headless=new",
        "--disable-gpu",
        "--hide-scrollbars",
        "--force-device-scale-factor=2",
        "--window-size=1200,1600",
        f"--screenshot={png_path}",
        html_path,
    ]
    subprocess.run(cmd, check=True)
    os.remove(html_path)
    print("Rendered:", png_path)
