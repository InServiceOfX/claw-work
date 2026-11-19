/**
 * ffmpeg / asciinema Command Builder
 * 
 * Generate copy-paste commands for converting terminal recordings and
 * screen recordings to 9:16 vertical videos for social media.
 * 
 * MIT License - see repository LICENSE
 */

(function() {
  'use strict';

  const OUTPUT_WIDTH = 1080;
  const OUTPUT_HEIGHT = 1920;
  const OUTPUT_FPS = 30;

  function escapeShellArg(arg) {
    if (/^[a-zA-Z0-9._\/-]+$/.test(arg)) {
      return arg;
    }
    return `"${arg.replace(/"/g, '\\"')}"`;
  }

  function buildAggCommand(options) {
    const {
      inputFile,
      outputFile = 'terminal.gif',
      cols = 80,
      rows = 24,
      fontSize = 14,
      theme = 'monokai'
    } = options;

    const args = [
      'agg',
      `--cols ${cols}`,
      `--rows ${rows}`,
      `--font-size ${fontSize}`
    ];

    if (theme && theme !== 'default') {
      args.push(`--theme ${theme}`);
    }

    args.push(escapeShellArg(inputFile));
    args.push(escapeShellArg(outputFile));

    return args.join(' \\\n  ');
  }

  function buildGifToMp4Command(options) {
    const {
      inputFile,
      outputFile = 'terminal.mp4',
      width = OUTPUT_WIDTH,
      height = OUTPUT_HEIGHT,
      fps = OUTPUT_FPS,
      bgColor = 'black'
    } = options;

    const scaleFilter = `scale=${width}:${height}:force_original_aspect_ratio=decrease,` +
      `pad=${width}:${height}:(ow-iw)/2:(oh-ih)/2:${bgColor}`;

    return [
      'ffmpeg -y',
      `-i ${escapeShellArg(inputFile)}`,
      '-movflags faststart',
      '-pix_fmt yuv420p',
      `-vf "${scaleFilter}"`,
      `-r ${fps}`,
      escapeShellArg(outputFile)
    ].join(' \\\n  ');
  }

  function buildScaleAndPadCommand(options) {
    const {
      inputFile,
      outputFile = 'output.mp4',
      width = OUTPUT_WIDTH,
      height = OUTPUT_HEIGHT,
      fps = OUTPUT_FPS,
      bgColor = 'black'
    } = options;

    const scaleFilter = `scale=${width}:${height}:force_original_aspect_ratio=decrease,` +
      `pad=${width}:${height}:(ow-iw)/2:(oh-ih)/2:${bgColor}`;

    return [
      'ffmpeg -y',
      `-i ${escapeShellArg(inputFile)}`,
      `-vf "${scaleFilter}"`,
      '-c:v libx264',
      '-c:a aac',
      '-pix_fmt yuv420p',
      `-r ${fps}`,
      escapeShellArg(outputFile)
    ].join(' \\\n  ');
  }

  function buildSubtitleBurnCommand(options) {
    const {
      inputFile,
      captionFile,
      outputFile = 'output_captioned.mp4',
      fontSize = 24,
      fontColor = 'white',
      outlineColor = 'black',
      outlineWidth = 2,
      marginV = 100
    } = options;

    const subtitleStyle = [
      `FontSize=${fontSize}`,
      `PrimaryColour=&H00${fontColor === 'white' ? 'FFFFFF' : 'FFFF00'}&`,
      `OutlineColour=&H00000000&`,
      `Outline=${outlineWidth}`,
      `MarginV=${marginV}`
    ].join(',');

    const subtitleFilter = `subtitles=${escapeShellArg(captionFile)}:force_style='${subtitleStyle}'`;

    return [
      'ffmpeg -y',
      `-i ${escapeShellArg(inputFile)}`,
      `-vf "${subtitleFilter}"`,
      '-c:v libx264',
      '-c:a copy',
      '-pix_fmt yuv420p',
      escapeShellArg(outputFile)
    ].join(' \\\n  ');
  }

  function buildWhisperCommand(options) {
    const {
      inputFile,
      outputDir = '.',
      model = 'base',
      language = 'en',
      outputFormat = 'srt'
    } = options;

    return [
      'whisper',
      escapeShellArg(inputFile),
      `--model ${model}`,
      `--language ${language}`,
      `--output_format ${outputFormat}`,
      `--output_dir ${escapeShellArg(outputDir)}`
    ].join(' \\\n  ');
  }

  function buildTrimCommand(options) {
    const {
      inputFile,
      outputFile = 'trimmed.mp4',
      startTime = '00:00:00',
      duration
    } = options;

    const args = [
      'ffmpeg -y',
      `-ss ${startTime}`,
      `-i ${escapeShellArg(inputFile)}`
    ];

    if (duration) {
      args.push(`-t ${duration}`);
    }

    args.push('-c copy');
    args.push(escapeShellArg(outputFile));

    return args.join(' \\\n  ');
  }

  function buildFullPipelineCommands(options) {
    const {
      inputType,
      inputFile,
      outputName = 'short',
      captionFile,
      generateCaptions,
      trim,
      startTime,
      duration
    } = options;

    const commands = [];

    if (inputType === 'cast') {
      commands.push({
        step: 1,
        description: 'Convert asciinema cast to GIF',
        command: buildAggCommand({
          inputFile,
          outputFile: `${outputName}_terminal.gif`,
          cols: 60,
          rows: 40,
          fontSize: 24
        })
      });

      commands.push({
        step: 2,
        description: 'Convert GIF to vertical MP4',
        command: buildGifToMp4Command({
          inputFile: `${outputName}_terminal.gif`,
          outputFile: `${outputName}_vertical.mp4`
        })
      });
    } else {
      let currentInput = inputFile;
      let stepNum = 1;

      if (trim && (startTime || duration)) {
        commands.push({
          step: stepNum++,
          description: 'Trim video',
          command: buildTrimCommand({
            inputFile: currentInput,
            outputFile: `${outputName}_trimmed.mp4`,
            startTime: startTime || '00:00:00',
            duration
          })
        });
        currentInput = `${outputName}_trimmed.mp4`;
      }

      commands.push({
        step: stepNum++,
        description: 'Scale and pad to 9:16 vertical',
        command: buildScaleAndPadCommand({
          inputFile: currentInput,
          outputFile: `${outputName}_vertical.mp4`
        })
      });
      currentInput = `${outputName}_vertical.mp4`;

      if (generateCaptions) {
        commands.push({
          step: stepNum++,
          description: 'Generate captions with Whisper (optional)',
          command: buildWhisperCommand({
            inputFile: currentInput,
            outputDir: '.'
          })
        });
      }

      if (captionFile) {
        commands.push({
          step: stepNum++,
          description: 'Burn in captions',
          command: buildSubtitleBurnCommand({
            inputFile: currentInput,
            captionFile,
            outputFile: `${outputName}_captioned.mp4`
          })
        });
      }
    }

    return commands;
  }

  function copyToClipboard(text, button) {
    navigator.clipboard.writeText(text).then(() => {
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      setTimeout(() => { button.textContent = originalText; }, 1500);
    }).catch(() => {
      const textArea = document.createElement('textarea');
      textArea.value = text;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
      const originalText = button.textContent;
      button.textContent = 'Copied!';
      setTimeout(() => { button.textContent = originalText; }, 1500);
    });
  }

  function init() {
    const form = document.getElementById('command-form');
    const outputContainer = document.getElementById('command-output');
    const inputTypeSelect = document.getElementById('input-type');
    const castOptions = document.getElementById('cast-options');
    const videoOptions = document.getElementById('video-options');

    if (!form) return;

    function toggleOptions() {
      const type = inputTypeSelect.value;
      if (castOptions) castOptions.style.display = type === 'cast' ? 'block' : 'none';
      if (videoOptions) videoOptions.style.display = type === 'video' ? 'block' : 'none';
    }

    inputTypeSelect?.addEventListener('change', toggleOptions);
    toggleOptions();

    form.addEventListener('submit', (e) => {
      e.preventDefault();
      
      const formData = new FormData(form);
      const options = {
        inputType: formData.get('input-type') || 'video',
        inputFile: formData.get('input-file') || 'input.mp4',
        outputName: formData.get('output-name') || 'short',
        captionFile: formData.get('caption-file') || '',
        generateCaptions: formData.get('generate-captions') === 'on',
        trim: formData.get('trim') === 'on',
        startTime: formData.get('start-time') || '',
        duration: formData.get('duration') || ''
      };

      const commands = buildFullPipelineCommands(options);

      outputContainer.innerHTML = commands.map(cmd => `
        <div class="card">
          <div class="card-header">
            <h4 class="card-title">Step ${cmd.step}: ${cmd.description}</h4>
          </div>
          <div class="output-block">
            <pre><code>${cmd.command}</code></pre>
            <button class="btn btn-sm copy-btn">Copy</button>
          </div>
        </div>
      `).join('');

      outputContainer.querySelectorAll('.card').forEach(card => {
        const code = card.querySelector('code').textContent;
        const btn = card.querySelector('.copy-btn');
        btn.addEventListener('click', () => copyToClipboard(code, btn));
      });

      const allCommands = commands.map(c => `# Step ${c.step}: ${c.description}\n${c.command}`).join('\n\n');
      
      const copyAllBtn = document.createElement('button');
      copyAllBtn.className = 'btn btn-primary mt-1';
      copyAllBtn.textContent = 'Copy All Commands';
      copyAllBtn.addEventListener('click', () => copyToClipboard(allCommands, copyAllBtn));
      outputContainer.appendChild(copyAllBtn);
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
      buildAggCommand,
      buildGifToMp4Command,
      buildScaleAndPadCommand,
      buildSubtitleBurnCommand,
      buildWhisperCommand,
      buildTrimCommand,
      buildFullPipelineCommands,
      OUTPUT_WIDTH,
      OUTPUT_HEIGHT,
      OUTPUT_FPS
    };
  }
})();
