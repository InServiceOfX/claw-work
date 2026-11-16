/**
 * Launch Shorts Planner
 * 
 * Generates a 5-Short launch plan from a README or product blurb.
 * Outputs hook options, shot list, captions, and claw_shorts.py commands.
 * 
 * MIT License - see repository LICENSE
 */

(function() {
  'use strict';

  const OUTPUT_WIDTH = 1080;
  const OUTPUT_HEIGHT = 1920;
  const OUTPUT_FPS = 30;
  const SAFE_TOP = 150;
  const SAFE_BOTTOM = 300;

  const HOOK_PATTERNS = [
    { pattern: /stop.+scroll/i, type: 'interruption' },
    { pattern: /you('re|r)?\s+(doing|making)/i, type: 'challenge' },
    { pattern: /why\s+.+\s+(fail|wrong|mistake)/i, type: 'problem' },
    { pattern: /secret|hidden|trick/i, type: 'curiosity' },
    { pattern: /(\d+)\s*(second|minute|step|way)/i, type: 'listicle' }
  ];

  const TEMPLATES = {
    hook_reveal: {
      description: 'Hook card (0-2s), content, CTA card (end)',
      flags: ['--hook', '--cta', '--title']
    },
    step_breakdown: {
      description: 'Numbered step overlays with paced captions',
      flags: ['--steps', '--cta', '--title']
    },
    demo_zoom: {
      description: 'Keyframed zoom/pan on a screen recording',
      flags: ['--hook', '--cta', '--title', '--zoom']
    },
    terminal_typer: {
      description: 'Asciinema cast rendered to frames',
      flags: ['--cast', '--hook', '--cta', '--title']
    },
    readme_scroll: {
      description: 'Scrolling README capture',
      flags: ['--url', '--hook', '--cta', '--title', '--scroll-duration']
    }
  };

  function extractProjectInfo(text) {
    const lines = text.split('\n');
    let name = '';
    let tagline = '';
    let features = [];
    let installCmd = '';
    let usageSnippets = [];

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      
      if (!name && line.startsWith('# ')) {
        name = line.replace(/^#\s*/, '').replace(/[^\w\s-]/g, '').trim();
      }
      
      if (name && !tagline && line && !line.startsWith('#') && !line.startsWith('```')) {
        if (line.length > 10 && line.length < 200) {
          tagline = line;
        }
      }

      if (/^#{2,3}\s*(feature|what|why)/i.test(line)) {
        for (let j = i + 1; j < Math.min(i + 15, lines.length); j++) {
          const featureLine = lines[j].trim();
          if (featureLine.startsWith('- ') || featureLine.startsWith('* ')) {
            features.push(featureLine.replace(/^[-*]\s*/, '').replace(/\*\*/g, ''));
          }
          if (featureLine.startsWith('#')) break;
        }
      }

      if (/^#{2,3}\s*(install|getting\s*started|quick\s*start)/i.test(line)) {
        for (let j = i + 1; j < Math.min(i + 10, lines.length); j++) {
          const cmdLine = lines[j].trim();
          if (cmdLine.startsWith('```')) {
            const nextLine = lines[j + 1] ? lines[j + 1].trim() : '';
            if (nextLine && !nextLine.startsWith('```')) {
              installCmd = nextLine;
            }
            break;
          }
          if (/^(npm|pip|cargo|brew|apt|yarn|pnpm)\s+/i.test(cmdLine)) {
            installCmd = cmdLine;
            break;
          }
        }
      }

      if (/^#{2,3}\s*(usage|example|how\s*to)/i.test(line)) {
        let inCodeBlock = false;
        let snippet = [];
        for (let j = i + 1; j < Math.min(i + 20, lines.length); j++) {
          const usageLine = lines[j];
          if (usageLine.trim().startsWith('```')) {
            if (inCodeBlock) {
              if (snippet.length > 0) {
                usageSnippets.push(snippet.join('\n'));
              }
              snippet = [];
            }
            inCodeBlock = !inCodeBlock;
            continue;
          }
          if (inCodeBlock) {
            snippet.push(usageLine);
          }
          if (usageLine.trim().startsWith('#') && !inCodeBlock) break;
        }
      }
    }

    if (!name) {
      const firstWords = text.split(/\s+/).slice(0, 5).join(' ');
      name = firstWords.substring(0, 30);
    }

    return {
      name: name.trim(),
      tagline: tagline.trim(),
      features: features.slice(0, 5),
      installCmd: installCmd.trim(),
      usageSnippets: usageSnippets.slice(0, 3)
    };
  }

  function generateHooks(projectInfo) {
    const { name, tagline, features } = projectInfo;
    const hooks = [];
    
    hooks.push({
      text: `${name} in 30 seconds`,
      type: 'direct',
      rationale: 'Clear promise with time constraint'
    });

    if (tagline) {
      const shortTagline = tagline.length > 50 
        ? tagline.substring(0, 47) + '...'
        : tagline;
      hooks.push({
        text: shortTagline,
        type: 'value-prop',
        rationale: 'Project tagline as hook'
      });
    }

    if (features.length > 0) {
      const count = Math.min(features.length, 5);
      hooks.push({
        text: `${count} reasons to try ${name}`,
        type: 'listicle',
        rationale: 'Listicle format with specific count'
      });
    }

    hooks.push({
      text: `You need ${name}`,
      type: 'direct',
      rationale: 'Direct statement creates curiosity'
    });

    if (features.length > 0) {
      const feature = features[0].substring(0, 40);
      hooks.push({
        text: feature.endsWith('.') ? feature : feature,
        type: 'feature',
        rationale: 'Lead with top feature'
      });
    }

    return hooks.slice(0, 5);
  }

  function generateShotList(projectInfo, selectedHook) {
    const { name, tagline, features, installCmd, usageSnippets } = projectInfo;
    const shots = [];

    shots.push({
      short: 1,
      title: 'Hook + Overview',
      template: 'hook_reveal',
      duration: '15-30s',
      hook: selectedHook || `${name} in 30 seconds`,
      content: `Show logo/name, then ${tagline || 'quick overview'}`,
      cta: `Follow for more ${name} tips`,
      caption: `${name}. ${tagline || 'Check it out.'}`
    });

    shots.push({
      short: 2,
      title: 'Installation',
      template: installCmd ? 'terminal_typer' : 'step_breakdown',
      duration: '20-45s',
      hook: `Installing ${name}`,
      content: installCmd 
        ? `Terminal: ${installCmd}`
        : 'Show installation steps',
      cta: 'Save for later',
      caption: `How to install ${name}. ${installCmd || 'Follow the steps.'}`
    });

    const demoFeatures = features.slice(0, 2);
    shots.push({
      short: 3,
      title: 'Quick Demo',
      template: 'demo_zoom',
      duration: '30-60s',
      hook: `${name} demo`,
      content: usageSnippets[0] 
        ? `Demo: ${usageSnippets[0].substring(0, 100)}`
        : `Show ${name} in action`,
      cta: 'Try it yourself',
      caption: `${name} in action.${demoFeatures[0] ? ' ' + demoFeatures[0] : ''}`
    });

    if (features.length >= 3) {
      shots.push({
        short: 4,
        title: 'Top Features',
        template: 'step_breakdown',
        duration: '30-45s',
        hook: `${features.length} ${name} features`,
        content: `Features: ${features.slice(0, 3).join(', ')}`,
        steps: features.slice(0, 3),
        cta: 'Which one is your favorite?',
        caption: `Top features of ${name}: ${features.slice(0, 3).join('. ')}`
      });
    } else {
      shots.push({
        short: 4,
        title: 'Use Case',
        template: 'hook_reveal',
        duration: '20-30s',
        hook: `When to use ${name}`,
        content: 'Show a specific use case',
        cta: 'Share your use case',
        caption: `When to use ${name}. Perfect for your next project.`
      });
    }

    shots.push({
      short: 5,
      title: 'CTA + Links',
      template: 'readme_scroll',
      duration: '15-30s',
      hook: `Get ${name}`,
      content: 'README scroll showing where to get it',
      cta: 'Link in bio',
      caption: `Get ${name} now. Link in bio. #opensource #devtools`
    });

    return shots;
  }

  function generateCommand(shot, projectName) {
    const script = 'python3 claw_shorts.py';
    const flags = [];
    
    flags.push('render');
    flags.push('--template', shot.template);
    flags.push('--title', `"${shot.title}"`);
    flags.push('--hook', `"${shot.hook}"`);
    flags.push('--cta', `"${shot.cta}"`);
    
    if (shot.template === 'step_breakdown' && shot.steps) {
      flags.push('--steps', `"${shot.steps.join(',')}"`);
    }
    
    if (shot.template === 'demo_zoom') {
      flags.push('--zoom', '1.5');
    }

    if (shot.template === 'terminal_typer') {
      flags.push('--cast', `"${projectName.toLowerCase().replace(/\s+/g, '-')}.cast"`);
    }

    if (shot.template === 'readme_scroll') {
      flags.push('--url', `"https://github.com/OWNER/${projectName.toLowerCase().replace(/\s+/g, '-')}"`);
      flags.push('--scroll-duration', '8');
    }

    flags.push('--input', `"input/short${shot.short}/"`);
    flags.push('--output', `"output/"`);
    flags.push('--seed', shot.short.toString());
    
    return `${script} ${flags.join(' ')}`;
  }

  function generatePlanJSON(projectInfo, selectedHook, shots) {
    const plan = {
      version: '1.0.0',
      generated_at: new Date().toISOString(),
      project: {
        name: projectInfo.name,
        tagline: projectInfo.tagline,
        features: projectInfo.features,
        install_command: projectInfo.installCmd
      },
      selected_hook: selectedHook,
      output_spec: {
        width: OUTPUT_WIDTH,
        height: OUTPUT_HEIGHT,
        fps: OUTPUT_FPS,
        safe_top: SAFE_TOP,
        safe_bottom: SAFE_BOTTOM
      },
      shorts: shots.map(shot => ({
        short_number: shot.short,
        title: shot.title,
        template: shot.template,
        duration: shot.duration,
        hook_text: shot.hook,
        cta_text: shot.cta,
        caption: shot.caption,
        content_notes: shot.content,
        steps: shot.steps || null,
        command: generateCommand(shot, projectInfo.name)
      })),
      suggested_hashtags: [
        '#shorts',
        '#devtools',
        '#opensource',
        `#${projectInfo.name.toLowerCase().replace(/[^a-z0-9]/g, '')}`,
        '#coding',
        '#tutorial'
      ],
      notes: {
        pre_render: 'Record or gather source clips before running commands',
        audio: 'Check audio licensing before monetized upload',
        verification: 'Review each render before posting'
      }
    };
    return plan;
  }

  function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  function copyToClipboard(text, button) {
    navigator.clipboard.writeText(text).then(() => {
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      setTimeout(() => {
        button.textContent = originalText;
      }, 1500);
    }).catch(() => {
      const textArea = document.createElement('textarea');
      textArea.value = text;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      setTimeout(() => {
        button.textContent = originalText;
      }, 1500);
    });
  }

  function init() {
    const form = document.getElementById('planner-form');
    const readmeInput = document.getElementById('readme-input');
    const resultsSection = document.getElementById('results');
    const hooksContainer = document.getElementById('hooks-container');
    const shotListContainer = document.getElementById('shot-list-container');
    const captionsContainer = document.getElementById('captions-container');
    const commandsContainer = document.getElementById('commands-container');
    const downloadBtn = document.getElementById('download-json');

    if (!form) return;

    let currentPlan = null;
    let projectInfo = null;
    let selectedHookIndex = 0;

    function updateResults() {
      if (!projectInfo) return;

      const hooks = generateHooks(projectInfo);
      const selectedHook = hooks[selectedHookIndex]?.text || hooks[0]?.text;
      const shots = generateShotList(projectInfo, selectedHook);
      currentPlan = generatePlanJSON(projectInfo, selectedHook, shots);

      hooksContainer.innerHTML = `
        <ul class="hook-list">
          ${hooks.map((h, i) => `
            <li class="${i === selectedHookIndex ? 'selected' : ''}" data-index="${i}">
              <strong>${escapeHtml(h.text)}</strong>
              <div class="text-sm text-muted">${escapeHtml(h.rationale)} (${h.type})</div>
            </li>
          `).join('')}
        </ul>
      `;

      hooksContainer.querySelectorAll('li').forEach(li => {
        li.addEventListener('click', () => {
          selectedHookIndex = parseInt(li.dataset.index, 10);
          updateResults();
        });
      });

      shotListContainer.innerHTML = `
        <table class="shot-list">
          <thead>
            <tr>
              <th>#</th>
              <th>Title</th>
              <th>Template</th>
              <th>Duration</th>
              <th>Hook</th>
            </tr>
          </thead>
          <tbody>
            ${shots.map(s => `
              <tr>
                <td>${s.short}</td>
                <td>${escapeHtml(s.title)}</td>
                <td><code>${s.template}</code></td>
                <td>${s.duration}</td>
                <td>${escapeHtml(s.hook)}</td>
              </tr>
            `).join('')}
          </tbody>
        </table>
      `;

      captionsContainer.innerHTML = shots.map(s => `
        <div class="card">
          <div class="card-header">
            <h4 class="card-title">Short ${s.short}: ${escapeHtml(s.title)}</h4>
          </div>
          <p>${escapeHtml(s.caption)}</p>
          <button class="btn btn-sm copy-btn" data-text="${escapeHtml(s.caption)}">Copy</button>
        </div>
      `).join('');

      captionsContainer.querySelectorAll('.copy-btn').forEach(btn => {
        btn.addEventListener('click', () => {
          copyToClipboard(btn.dataset.text, btn);
        });
      });

      commandsContainer.innerHTML = shots.map(s => {
        const cmd = generateCommand(s, projectInfo.name);
        return `
          <div class="output-block">
            <pre><code>${escapeHtml(cmd)}</code></pre>
            <button class="btn btn-sm copy-btn">Copy</button>
          </div>
        `;
      }).join('');

      commandsContainer.querySelectorAll('.output-block').forEach(block => {
        const code = block.querySelector('code').textContent;
        const btn = block.querySelector('.copy-btn');
        btn.addEventListener('click', () => copyToClipboard(code, btn));
      });

      resultsSection.style.display = 'block';
    }

    form.addEventListener('submit', (e) => {
      e.preventDefault();
      const text = readmeInput.value.trim();
      if (!text) return;
      
      projectInfo = extractProjectInfo(text);
      selectedHookIndex = 0;
      updateResults();
    });

    downloadBtn?.addEventListener('click', () => {
      if (!currentPlan) return;
      const json = JSON.stringify(currentPlan, null, 2);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `launch-plan-${currentPlan.project.name.toLowerCase().replace(/[^a-z0-9]/g, '-')}.json`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    });
  }

  if (typeof window !== 'undefined') {
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', init);
    } else {
      init();
    }
  }

  if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
      extractProjectInfo,
      generateHooks,
      generateShotList,
      generateCommand,
      generatePlanJSON,
      OUTPUT_WIDTH,
      OUTPUT_HEIGHT,
      OUTPUT_FPS,
      SAFE_TOP,
      SAFE_BOTTOM
    };
  }
})();
