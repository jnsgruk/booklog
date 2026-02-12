const escBar = (s) =>
  s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");

class BarChart extends HTMLElement {
  static get observedAttributes() {
    return ["data-items"];
  }

  connectedCallback() {
    requestAnimationFrame(() => this.render());
    this._themeObserver = new MutationObserver(() =>
      requestAnimationFrame(() => this.render()),
    );
    this._themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
  }

  disconnectedCallback() {
    this._themeObserver?.disconnect();
    this._themeObserver = null;
  }

  attributeChangedCallback() {
    if (this.isConnected) requestAnimationFrame(() => this.render());
  }

  render() {
    const raw = this.dataset.items || "";
    if (!raw) {
      this.innerHTML = "";
      return;
    }

    const items = raw
      .split("|")
      .map((s) => {
        const parts = s.split(":");
        if (parts.length < 3) return null;
        const label = parts[0].trim();
        const v1 = parseInt(parts[1], 10) || 0;
        const v2 = parseInt(parts[2], 10) || 0;
        return label ? { label, v1, v2 } : null;
      })
      .filter(Boolean);

    const allZero = items.every((i) => i.v1 === 0 && i.v2 === 0);
    if (items.length === 0 || allZero) {
      this.innerHTML = "";
      return;
    }

    const label1 = this.getAttribute("data-label-1") || "Series 1";
    const label2 = this.getAttribute("data-label-2") || "Series 2";

    const rgb =
      getComputedStyle(document.documentElement)
        .getPropertyValue("--highlight-rgb")
        .trim() || "185, 28, 28";

    const colorSolid = `rgba(${rgb}, 1)`;
    const colorLight = `rgba(${rgb}, 0.35)`;

    // Chart dimensions
    const svgW = 600;
    const svgH = 300;
    const padTop = 20;
    const padBottom = 48;
    const padLeft = 52;
    const padRight = 56;
    const chartW = svgW - padLeft - padRight;
    const chartH = svgH - padTop - padBottom;

    const max1 = Math.max(...items.map((i) => i.v1), 1);
    const max2 = Math.max(...items.map((i) => i.v2), 1);

    // Compute nice tick values
    const niceTicks = (maxVal, count) => {
      if (maxVal === 0) return [0];
      const rough = maxVal / (count - 1);
      const mag = Math.pow(10, Math.floor(Math.log10(rough)));
      const candidates = [1, 2, 2.5, 5, 10];
      const step = mag * (candidates.find((c) => c * mag >= rough) || 10);
      const ticks = [];
      for (let v = 0; v <= maxVal + step * 0.01; v += step) {
        ticks.push(Math.round(v * 100) / 100);
      }
      if (ticks[ticks.length - 1] < maxVal) {
        ticks.push(ticks[ticks.length - 1] + step);
      }
      return ticks;
    };

    const ticks1 = niceTicks(max1, 5);
    const ticks2 = niceTicks(max2, 5);
    const ceilMax1 = ticks1[ticks1.length - 1] || 1;
    const ceilMax2 = ticks2[ticks2.length - 1] || 1;

    // Bar geometry
    const n = items.length;
    const groupGap = Math.max(4, (chartW / n) * 0.3);
    const groupW = (chartW - groupGap * (n - 1)) / n;
    const barGap = Math.max(2, groupW * 0.1);
    const barW = (groupW - barGap) / 2;

    // Grid lines
    const gridLines = ticks1.map((val) => {
      const y = padTop + chartH - (val / ceilMax1) * chartH;
      return `<line x1="${padLeft}" x2="${padLeft + chartW}" y1="${y}" y2="${y}" stroke="var(--border)" stroke-width="0.5" />`;
    });

    // Y-axis left labels (series 1)
    const yLabelsLeft = ticks1.map((val) => {
      const y = padTop + chartH - (val / ceilMax1) * chartH;
      return `<text x="${padLeft - 8}" y="${y}" text-anchor="end" dominant-baseline="middle" class="text-text-muted" style="font-size:10px">${val}</text>`;
    });

    // Y-axis right labels (series 2)
    const yLabelsRight = ticks2.map((val) => {
      const y = padTop + chartH - (val / ceilMax2) * chartH;
      return `<text x="${padLeft + chartW + 8}" y="${y}" text-anchor="start" dominant-baseline="middle" class="text-text-muted" style="font-size:10px">${val}</text>`;
    });

    // Bars and x-axis labels
    const bars = [];
    const xLabels = [];

    items.forEach((item, i) => {
      const groupX = padLeft + i * (groupW + groupGap);

      // Series 1 bar (left)
      const h1 = (item.v1 / ceilMax1) * chartH;
      const y1 = padTop + chartH - h1;
      bars.push(
        `<rect x="${groupX}" y="${y1}" width="${barW}" height="${h1}" rx="2" fill="${colorSolid}" />`,
      );

      // Series 2 bar (right)
      const h2 = (item.v2 / ceilMax2) * chartH;
      const y2 = padTop + chartH - h2;
      bars.push(
        `<rect x="${groupX + barW + barGap}" y="${y2}" width="${barW}" height="${h2}" rx="2" fill="${colorLight}" />`,
      );

      // X-axis label
      const labelX = groupX + groupW / 2;
      const labelY = padTop + chartH + 16;
      xLabels.push(
        `<text x="${labelX}" y="${labelY}" text-anchor="middle" class="text-text-muted" style="font-size:10px">${escBar(item.label)}</text>`,
      );
    });

    // Baseline
    const baseline = `<line x1="${padLeft}" x2="${padLeft + chartW}" y1="${padTop + chartH}" y2="${padTop + chartH}" stroke="var(--border)" stroke-width="1" />`;

    // Accessibility label
    const ariaLabel = `Monthly activity: ${items.map((i) => `${escBar(i.label)} ${i.v1} ${escBar(label1)}, ${i.v2} ${escBar(label2)}`).join("; ")}`;

    // Legend
    const legend = `<div style="display:flex;align-items:center;justify-content:center;gap:1.25rem;margin-top:0.5rem">
      <div style="display:flex;align-items:center;gap:0.375rem">
        <span style="flex-shrink:0;width:0.5rem;height:0.5rem;border-radius:9999px;background:${colorSolid}"></span>
        <span class="text-xs text-text-secondary">${escBar(label1)}</span>
      </div>
      <div style="display:flex;align-items:center;gap:0.375rem">
        <span style="flex-shrink:0;width:0.5rem;height:0.5rem;border-radius:9999px;background:${colorLight}"></span>
        <span class="text-xs text-text-secondary">${escBar(label2)}</span>
      </div>
    </div>`;

    this.innerHTML = `<div style="display:flex;flex-direction:column;align-items:center;width:100%">
      <svg viewBox="0 0 ${svgW} ${svgH}" style="display:block;width:100%;height:auto" role="img" aria-label="${ariaLabel}">
        ${gridLines.join("")}
        ${baseline}
        ${bars.join("")}
        ${yLabelsLeft.join("")}
        ${yLabelsRight.join("")}
        ${xLabels.join("")}
      </svg>
      ${legend}
    </div>`;
  }
}

customElements.define("bar-chart", BarChart);
