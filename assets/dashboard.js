const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const state = { data: null, range: "all", mode: "all", chartPoints: [], chartFrame: 0 };

const css = (name) => getComputedStyle(document.documentElement).getPropertyValue(name).trim();
const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
const number = (value, digits = 0) => Number(value || 0).toLocaleString(undefined, {
  minimumFractionDigits: digits,
  maximumFractionDigits: digits,
});
const shortDate = (milliseconds) => new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "2-digit",
}).format(new Date(milliseconds));
const relativeDate = (milliseconds) => {
  const days = Math.max(0, Math.floor((Date.now() - milliseconds) / 86400000));
  if (days === 0) return "today";
  if (days === 1) return "yesterday";
  if (days < 30) return `${days}d ago`;
  return shortDate(milliseconds);
};
const duration = (seconds) => {
  const hours = seconds / 3600;
  if (hours >= 1) return `${number(hours, hours < 10 ? 1 : 0)}h`;
  return `${number(seconds / 60, 0)}m`;
};

function setLoading(loading) {
  $("#refreshButton").classList.toggle("loading", loading);
  $("#refreshButton").textContent = loading ? "Reading data" : "Refresh data";
  const firstLoad = loading && !state.data;
  document.body.classList.toggle("is-loading", firstLoad);
  $("#loadingShell").hidden = !firstLoad;
  $("#main").setAttribute("aria-busy", String(loading));
}

async function loadData() {
  setLoading(true);
  $("#errorState").hidden = true;
  try {
    const response = await fetch("/api/dashboard", { cache: "no-store" });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || "Local data request failed");
    state.data = payload;
    render();
  } catch (error) {
    showError(error);
  } finally {
    setLoading(false);
  }
}

function showError(error) {
  $$("main > section").forEach((section) => { section.hidden = true; });
  $("#errorState").hidden = false;
  $("#errorMessage").textContent = error.message || String(error);
}

function render() {
  const { data } = state;
  const empty = data.overview.testsCompleted === 0;
  $("#globalEmpty").hidden = !empty;
  $$("main > section:not(.empty-state):not(.error-state)").forEach((section) => {
    section.hidden = empty;
  });
  if (empty) {
    requestAnimationFrame(revealVisible);
    return;
  }
  renderOverview();
  renderModeOptions();
  renderProgress();
  renderInsights();
  renderWrongWords();
  renderPatterns();
  renderActivity();
  requestAnimationFrame(revealVisible);
}

function renderOverview() {
  const o = state.data.overview;
  const unit = state.data.speedUnit;
  let title = "Your rhythm is becoming visible.";
  let summary = "Each saved test sharpens the next recommendation.";
  if (o.trendPercent >= 2) {
    title = "Your speed is climbing.";
    summary = `Recent pace is ${number(o.trendPercent, 1)}% higher than the previous ten-test block.`;
  } else if (o.trendPercent <= -2) {
    title = "Your speed needs recovery.";
    summary = "The recent dip is a signal to protect accuracy and review the test mix.";
  } else if (o.recentAccuracy < 96) {
    title = "Accuracy is the next unlock.";
    summary = `A ${number(o.speedLeak, 1)} ${unit} raw-to-net gap shows where usable speed is escaping.`;
  }
  $(".hero-number small").textContent = `recent ${unit}`;
  $(".leak-number small").textContent = `${unit} raw-to-net gap`;
  $("#heroTitle").textContent = title;
  $("#heroSummary").textContent = summary;
  $("#dataFreshness").textContent = `Updated ${new Date(state.data.generatedAtMs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
  animateValue($("#recentWpm"), o.recentWpm, 1);
  setMetric("highestWpm", number(o.highestWpm, 1));
  setMetric("averageAccuracy", `${number(o.averageAccuracy, 1)}%`);
  setMetric("testsCompleted", number(o.testsCompleted));
  setMetric("timeTypingSec", duration(o.timeTypingSec));
  setMetric("currentStreak", number(o.currentStreak));
  setMetric("speedLeak", number(o.speedLeak, 1));
  drawHeroChart();
}

function setMetric(name, value) {
  const element = $(`[data-metric="${name}"]`);
  if (element) element.textContent = value;
}

function animateValue(element, target, digits = 0) {
  if (reduceMotion) {
    element.textContent = number(target, digits);
    return;
  }
  const start = performance.now();
  const run = (now) => {
    const progress = clamp((now - start) / 900, 0, 1);
    const eased = 1 - Math.pow(1 - progress, 4);
    element.textContent = number(target * eased, digits);
    if (progress < 1) requestAnimationFrame(run);
  };
  requestAnimationFrame(run);
}

function prepareCanvas(canvas) {
  const ratio = Math.min(window.devicePixelRatio || 1, 2);
  const rect = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.floor(rect.width));
  const height = Math.max(1, Math.floor(rect.height));
  if (canvas.width !== width * ratio || canvas.height !== height * ratio) {
    canvas.width = width * ratio;
    canvas.height = height * ratio;
  }
  const context = canvas.getContext("2d");
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, width, height);
  return { context, width, height };
}

function drawHeroChart() {
  const canvas = $("#heroCanvas");
  const values = state.data.timeline.slice(-28).map((point) => point.wpm);
  const { context, width, height } = prepareCanvas(canvas);
  if (values.length < 2) return;
  const padding = Math.max(28, width * .065);
  const min = state.data.startGraphsAtZero ? 0 : Math.min(...values) * .82;
  const max = Math.max(...values) * 1.12;
  const x = (index) => padding + index / (values.length - 1) * (width - padding * 2);
  const y = (value) => height - padding - (value - min) / Math.max(1, max - min) * (height - padding * 2);
  const accent = css("--accent");
  const surface = css("--surface-solid");
  const durationMs = reduceMotion ? 1 : 1200;
  const started = performance.now();
  const frame = (now) => {
    const progress = clamp((now - started) / durationMs, 0, 1);
    context.clearRect(0, 0, width, height);
    context.save();
    context.beginPath();
    context.rect(0, 0, width * (1 - Math.pow(1 - progress, 3)), height);
    context.clip();
    const gradient = context.createLinearGradient(0, padding, 0, height);
    gradient.addColorStop(0, `${accent}42`);
    gradient.addColorStop(1, `${accent}00`);
    context.beginPath();
    values.forEach((value, index) => index ? context.lineTo(x(index), y(value)) : context.moveTo(x(index), y(value)));
    context.lineTo(x(values.length - 1), height);
    context.lineTo(x(0), height);
    context.closePath();
    context.fillStyle = gradient;
    context.fill();
    context.beginPath();
    values.forEach((value, index) => index ? context.lineTo(x(index), y(value)) : context.moveTo(x(index), y(value)));
    context.strokeStyle = accent;
    context.lineWidth = 3;
    context.lineJoin = "round";
    context.stroke();
    const last = values.length - 1;
    context.beginPath();
    context.arc(x(last), y(values[last]), 6, 0, Math.PI * 2);
    context.fillStyle = surface;
    context.fill();
    context.lineWidth = 3;
    context.strokeStyle = accent;
    context.stroke();
    context.restore();
    if (progress < 1) requestAnimationFrame(frame);
  };
  requestAnimationFrame(frame);
}

function renderModeOptions() {
  const select = $("#modeFilter");
  const current = select.value;
  const modes = [...new Set(state.data.timeline.map((point) => point.mode))];
  select.innerHTML = `<option value="all">All modes</option>${modes.map((mode) => `<option value="${escapeHtml(mode)}">${escapeHtml(labelMode(mode))}</option>`).join("")}`;
  select.value = modes.includes(current) ? current : "all";
  state.mode = select.value;
}

function labelMode(mode) {
  return mode.charAt(0).toUpperCase() + mode.slice(1);
}

function filteredTimeline() {
  let points = state.data.timeline;
  if (state.mode !== "all") points = points.filter((point) => point.mode === state.mode);
  if (state.range !== "all") points = points.slice(-Number(state.range));
  return points;
}

function renderProgress() {
  const points = filteredTimeline();
  $("#progressEmpty").hidden = points.length >= 2;
  $(".legend-net").textContent = `Net ${state.data.speedUnit}`;
  $(".legend-raw").textContent = `Raw ${state.data.speedUnit}`;
  const trend = state.data.overview.trendPercent;
  $("#trendLabel").textContent = trend == null
    ? "More history needed"
    : trend >= 0
      ? `${number(trend, 1)}% recent gain`
      : `${number(Math.abs(trend), 1)}% recent decline`;
  drawProgressChart(points);
}

function drawProgressChart(points) {
  cancelAnimationFrame(state.chartFrame);
  const canvas = $("#progressCanvas");
  const { context, width, height } = prepareCanvas(canvas);
  state.chartPoints = [];
  if (points.length < 2) return;
  const padding = { top: 48, right: 52, bottom: 50, left: 58 };
  const plotWidth = width - padding.left - padding.right;
  const plotHeight = height - padding.top - padding.bottom;
  const maxSpeed = Math.max(...points.flatMap((point) => [point.wpm, point.rawWpm]), 10) * 1.12;
  const minSpeed = state.data.startGraphsAtZero
    ? 0
    : Math.max(0, Math.min(...points.map((point) => point.wpm)) * .8);
  const x = (index) => padding.left + index / (points.length - 1) * plotWidth;
  const speedY = (value) => padding.top + (1 - (value - minSpeed) / Math.max(1, maxSpeed - minSpeed)) * plotHeight;
  const accuracyY = (value) => padding.top + (1 - (value - 80) / 20) * plotHeight;
  state.chartPoints = points.map((point, index) => ({ x: x(index), y: speedY(point.wpm), point }));
  const started = performance.now();
  const durationMs = reduceMotion ? 1 : 1050;
  const frame = (now) => {
    const progress = clamp((now - started) / durationMs, 0, 1);
    context.clearRect(0, 0, width, height);
    drawChartGrid(context, width, height, padding, minSpeed, maxSpeed);
    context.save();
    context.beginPath();
    context.rect(padding.left, padding.top, plotWidth * (1 - Math.pow(1 - progress, 3)), plotHeight + 4);
    context.clip();
    drawLine(context, points.map((point, index) => [x(index), speedY(point.rawWpm)]), css("--raw"), 1.5, []);
    drawLine(context, points.map((point, index) => [x(index), accuracyY(point.accuracy)]), css("--positive"), 1.5, [5, 6]);
    drawLine(context, points.map((point, index) => [x(index), speedY(point.wpm)]), css("--accent"), 3, []);
    context.restore();
    if (progress < 1) state.chartFrame = requestAnimationFrame(frame);
  };
  state.chartFrame = requestAnimationFrame(frame);
}

function drawChartGrid(context, width, height, padding, min, max) {
  const line = css("--line");
  const muted = css("--faint");
  context.font = `10px ${css("--font-mono")}`;
  context.fillStyle = muted;
  context.textAlign = "right";
  context.textBaseline = "middle";
  for (let index = 0; index <= 4; index += 1) {
    const y = padding.top + index / 4 * (height - padding.top - padding.bottom);
    context.beginPath();
    context.moveTo(padding.left, y);
    context.lineTo(width - padding.right, y);
    context.strokeStyle = line;
    context.lineWidth = 1;
    context.stroke();
    context.fillText(number(max - index / 4 * (max - min), 0), padding.left - 13, y);
  }
}

function drawLine(context, points, color, width, dash) {
  context.beginPath();
  points.forEach(([x, y], index) => index ? context.lineTo(x, y) : context.moveTo(x, y));
  context.strokeStyle = color;
  context.lineWidth = width;
  context.lineJoin = "round";
  context.lineCap = "round";
  context.setLineDash(dash);
  context.stroke();
  context.setLineDash([]);
}

function showChartTooltip(event) {
  if (!state.chartPoints.length) return;
  const canvas = $("#progressCanvas");
  const rect = canvas.getBoundingClientRect();
  const mouseX = event.clientX - rect.left;
  const nearest = state.chartPoints.reduce((best, point) => Math.abs(point.x - mouseX) < Math.abs(best.x - mouseX) ? point : best);
  const tooltip = $("#chartTooltip");
  tooltip.hidden = false;
  tooltip.style.left = `${nearest.x}px`;
  tooltip.style.top = `${nearest.y}px`;
  tooltip.innerHTML = `<strong>${number(nearest.point.wpm, 1)} ${escapeHtml(state.data.speedUnit)}</strong><span>${number(nearest.point.accuracy, 1)}% accuracy</span><span>${escapeHtml(labelMode(nearest.point.mode))} ${escapeHtml(nearest.point.mode2)}</span><span>${shortDate(nearest.point.timestampMs)}</span>`;
}

function renderInsights() {
  const stack = $("#insightStack");
  stack.innerHTML = state.data.insights.map((insight, index) => `
    <article class="insight-card" data-kind="${escapeHtml(insight.kind)}" data-index="${String(index + 1).padStart(2, "0")}">
      <h3>${escapeHtml(insight.title)}</h3>
      <p>${escapeHtml(insight.body)}</p>
      <strong>${escapeHtml(insight.action)}</strong>
    </article>`).join("");
  requestAnimationFrame(() => {
    $$(".insight-card", stack).forEach((card, index) => setTimeout(() => card.classList.add("visible"), reduceMotion ? 0 : index * 110));
  });
  const command = state.data.overview.weakWordCount ? "mtype practice missed --words 25" : "mtype practice slow --words 25";
  $("#practiceCommand code").textContent = command;
}

function renderWrongWords() {
  const query = $("#wordSearch").value.trim().toLowerCase();
  const words = state.data.wrongWords.filter((item) => item.word.toLowerCase().includes(query));
  $("#wordsEmpty").hidden = words.length > 0;
  const body = $("#wrongWordsBody");
  body.innerHTML = words.slice(0, 40).map((word, index) => `
    <tr data-word="${escapeHtml(word.word)}" class="${index === 0 ? "active" : ""}">
      <td>${escapeHtml(word.word)}</td>
      <td>${number(word.errorAttempts)}</td>
      <td>${number(word.errorRate, 1)}%</td>
      <td>${number(word.averageBurstWpm, 0)}</td>
      <td>${relativeDate(word.lastSeenMs)}</td>
    </tr>`).join("");
  $$(`tr`, body).forEach((row) => row.addEventListener("click", () => {
    $$(`tr`, body).forEach((item) => item.classList.remove("active"));
    row.classList.add("active");
    const word = state.data.wrongWords.find((item) => item.word === row.dataset.word);
    if (word) updateWordFocus(word);
  }));
  if (words[0]) updateWordFocus(words[0]);
  else resetWordFocus();
}

function updateWordFocus(word) {
  $("#focusWord").textContent = word.word;
  $("#focusRate").textContent = `${number(word.errorRate, 1)}%`;
  $("#focusAttempts").textContent = number(word.attempts);
  const variants = word.variants.filter((variant) => variant.typed !== word.word);
  $("#focusVariant").textContent = variants.length
    ? `Most common: ${variants.map((variant) => `'${variant.typed}' (${variant.count})`).join(", ")}.`
    : "Errors were corrected before submission, so the final word often looked clean.";
}

function resetWordFocus() {
  $("#focusWord").textContent = "Clean";
  $("#focusRate").textContent = "0%";
  $("#focusAttempts").textContent = "0";
  $("#focusVariant").textContent = "No matching word-level mistakes.";
}

function renderPatterns() {
  const confusions = $("#confusionGrid");
  confusions.innerHTML = state.data.confusions.slice(0, 16).map((item) => `
    <div class="confusion"><b>${escapeHtml(item.expected)} → ${escapeHtml(item.typed)}</b><small>${number(item.count)} times</small></div>`).join("");
  $("#confusionEmpty").hidden = state.data.confusions.length > 0;
  const slow = $("#slowWords");
  slow.innerHTML = state.data.slowWords.slice(0, 18).map((item) => `
    <span class="slow-word">${escapeHtml(item.word)} <b>${number(item.averageBurstWpm, 0)}</b></span>`).join("");
  $("#slowEmpty").hidden = state.data.slowWords.length > 0;
  $("#modeList").innerHTML = state.data.modes.slice(0, 8).map((item) => `
    <div class="mode-item"><span>${escapeHtml(item.mode)}</span><strong>${number(item.averageWpm, 1)}</strong><small>${number(item.tests)} tests, ${number(item.averageAccuracy, 1)}% acc</small></div>`).join("");
}

function renderActivity() {
  const today = Math.floor(Date.now() / 86400000);
  const start = today - 370;
  const counts = new Map(state.data.activity.map((item) => [item.day, item.count]));
  const max = Math.max(...counts.values(), 1);
  const cells = [];
  for (let day = start; day <= today; day += 1) {
    const count = counts.get(day) || 0;
    const level = count === 0 ? 0 : clamp(Math.ceil(count / max * 4), 1, 4);
    const date = new Date(day * 86400000);
    const dayLabel = date.toLocaleDateString(undefined, { timeZone: "UTC" });
    cells.push(`<span class="heat-cell" data-level="${level}" title="${dayLabel}: ${count} tests"></span>`);
  }
  $("#heatmap").innerHTML = cells.join("");
  $("#activityCopy").textContent = state.data.overview.currentStreak
    ? `You are on a ${state.data.overview.currentStreak}-day streak. Your longest is ${state.data.overview.longestStreak} days.`
    : `Your longest streak is ${state.data.overview.longestStreak} days. One test today starts the next run.`;
  renderMonthLabels(start, today);
}

function renderMonthLabels(start, today) {
  const labels = [];
  let lastMonth = -1;
  for (let day = start; day <= today; day += 7) {
    const date = new Date(day * 86400000);
    if (date.getUTCMonth() !== lastMonth) {
      lastMonth = date.getUTCMonth();
      labels.push(`<span>${date.toLocaleDateString(undefined, { month: "short", timeZone: "UTC" })}</span>`);
    }
  }
  $("#heatmapMonths").innerHTML = labels.join("");
}

function revealVisible() {
  if (reduceMotion) {
    $$(".reveal").forEach((element) => element.classList.add("visible"));
    return;
  }
  const observer = new IntersectionObserver((entries) => {
    entries.forEach((entry) => {
      if (entry.isIntersecting) {
        entry.target.classList.add("visible");
        observer.unobserve(entry.target);
      }
    });
  }, { threshold: .12 });
  $$(".reveal").forEach((element) => observer.observe(element));
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

$("#refreshButton").addEventListener("click", loadData);
$("#retryButton").addEventListener("click", loadData);
$("#wordSearch").addEventListener("input", renderWrongWords);
$("#modeFilter").addEventListener("change", (event) => {
  state.mode = event.target.value;
  renderProgress();
});
$$(`#rangeFilter button`).forEach((button) => button.addEventListener("click", () => {
  $$(`#rangeFilter button`).forEach((item) => item.classList.remove("active"));
  button.classList.add("active");
  state.range = button.dataset.range;
  renderProgress();
}));
$("#progressCanvas").addEventListener("mousemove", showChartTooltip);
$("#progressCanvas").addEventListener("mouseleave", () => { $("#chartTooltip").hidden = true; });
$("#copyPractice").addEventListener("click", async () => {
  const button = $("#copyPractice");
  try {
    await navigator.clipboard.writeText($("#practiceCommand code").textContent);
    button.textContent = "Copied";
  } catch {
    button.textContent = "Select command";
  }
  setTimeout(() => { button.textContent = "Copy"; }, 1400);
});

const resizeObserver = new ResizeObserver(() => {
  if (!state.data || state.data.overview.testsCompleted === 0) return;
  drawHeroChart();
  renderProgress();
});
resizeObserver.observe($("#heroCanvas"));
resizeObserver.observe($("#progressCanvas"));

loadData();
