/**
 * Tests for Launch Shorts Planner
 * 
 * Run with: node test_planner.js
 * 
 * Tests the core functions: extractProjectInfo, generateHooks,
 * generateShotList, generateCommand, and generatePlanJSON.
 * 
 * MIT License - see repository LICENSE
 */

const assert = require('assert');
const path = require('path');

const plannerPath = path.join(__dirname, '..', 'js', 'planner.js');
const planner = require(plannerPath);

const {
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
} = planner;

const SAMPLE_README = `# My CLI Tool

A fast command-line utility for processing files.

## Features

- Fast parallel processing
- Zero dependencies
- Cross-platform support
- Simple API

## Installation

\`\`\`bash
npm install my-cli-tool
\`\`\`

## Usage

\`\`\`bash
my-cli-tool --input file.txt --output result.json
\`\`\`

## License

MIT
`;

const MINIMAL_README = `# tiny

Does one thing.
`;

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`✓ ${name}`);
    passed++;
  } catch (error) {
    console.log(`✗ ${name}`);
    console.log(`  ${error.message}`);
    failed++;
  }
}

console.log('Testing planner.js\n');

test('Constants are defined', () => {
  assert.strictEqual(OUTPUT_WIDTH, 1080);
  assert.strictEqual(OUTPUT_HEIGHT, 1920);
  assert.strictEqual(OUTPUT_FPS, 30);
  assert.strictEqual(SAFE_TOP, 150);
  assert.strictEqual(SAFE_BOTTOM, 300);
});

test('extractProjectInfo extracts name from heading', () => {
  const info = extractProjectInfo(SAMPLE_README);
  assert.strictEqual(info.name, 'My CLI Tool');
});

test('extractProjectInfo extracts tagline', () => {
  const info = extractProjectInfo(SAMPLE_README);
  assert.strictEqual(info.tagline, 'A fast command-line utility for processing files.');
});

test('extractProjectInfo extracts features', () => {
  const info = extractProjectInfo(SAMPLE_README);
  assert.ok(Array.isArray(info.features));
  assert.ok(info.features.length >= 3);
  assert.ok(info.features.includes('Fast parallel processing'));
});

test('extractProjectInfo extracts install command', () => {
  const info = extractProjectInfo(SAMPLE_README);
  assert.strictEqual(info.installCmd, 'npm install my-cli-tool');
});

test('extractProjectInfo handles minimal README', () => {
  const info = extractProjectInfo(MINIMAL_README);
  assert.strictEqual(info.name, 'tiny');
  assert.strictEqual(info.tagline, 'Does one thing.');
});

test('extractProjectInfo handles empty input', () => {
  const info = extractProjectInfo('');
  assert.ok(info.name !== undefined);
  assert.ok(info.features !== undefined);
  assert.ok(Array.isArray(info.features));
});

test('generateHooks returns 5 hooks', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  assert.strictEqual(hooks.length, 5);
});

test('generateHooks includes project name', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const hasProjectName = hooks.some(h => h.text.includes('My CLI Tool'));
  assert.ok(hasProjectName, 'At least one hook should include project name');
});

test('generateHooks have required properties', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  hooks.forEach((hook, i) => {
    assert.ok(hook.text, `Hook ${i} should have text`);
    assert.ok(hook.type, `Hook ${i} should have type`);
    assert.ok(hook.rationale, `Hook ${i} should have rationale`);
  });
});

test('generateShotList returns 5 shots', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  assert.strictEqual(shots.length, 5);
});

test('generateShotList shots have required properties', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  shots.forEach((shot, i) => {
    assert.ok(shot.short, `Shot ${i} should have short number`);
    assert.ok(shot.title, `Shot ${i} should have title`);
    assert.ok(shot.template, `Shot ${i} should have template`);
    assert.ok(shot.duration, `Shot ${i} should have duration`);
    assert.ok(shot.hook, `Shot ${i} should have hook`);
    assert.ok(shot.cta, `Shot ${i} should have cta`);
    assert.ok(shot.caption, `Shot ${i} should have caption`);
  });
});

test('generateShotList uses valid templates', () => {
  const validTemplates = ['hook_reveal', 'step_breakdown', 'demo_zoom', 'terminal_typer', 'readme_scroll'];
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  shots.forEach(shot => {
    assert.ok(validTemplates.includes(shot.template), 
      `Template ${shot.template} should be valid`);
  });
});

test('generateCommand returns string', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  const cmd = generateCommand(shots[0], info.name);
  assert.ok(typeof cmd === 'string');
  assert.ok(cmd.length > 0);
});

test('generateCommand includes claw_shorts.py', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  const cmd = generateCommand(shots[0], info.name);
  assert.ok(cmd.includes('claw_shorts.py'));
});

test('generateCommand includes render and template', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const shots = generateShotList(info, 'Test hook');
  const cmd = generateCommand(shots[0], info.name);
  assert.ok(cmd.includes('render'));
  assert.ok(cmd.includes('--template'));
});

test('generatePlanJSON returns valid structure', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  assert.ok(plan.version);
  assert.ok(plan.generated_at);
  assert.ok(plan.project);
  assert.ok(plan.selected_hook);
  assert.ok(plan.output_spec);
  assert.ok(plan.shorts);
  assert.ok(plan.suggested_hashtags);
  assert.ok(plan.notes);
});

test('generatePlanJSON project has correct fields', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  assert.strictEqual(plan.project.name, 'My CLI Tool');
  assert.ok(plan.project.tagline);
  assert.ok(Array.isArray(plan.project.features));
  assert.ok(plan.project.install_command);
});

test('generatePlanJSON output_spec has correct dimensions', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  assert.strictEqual(plan.output_spec.width, 1080);
  assert.strictEqual(plan.output_spec.height, 1920);
  assert.strictEqual(plan.output_spec.fps, 30);
  assert.strictEqual(plan.output_spec.safe_top, 150);
  assert.strictEqual(plan.output_spec.safe_bottom, 300);
});

test('generatePlanJSON shorts have commands', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  assert.strictEqual(plan.shorts.length, 5);
  plan.shorts.forEach((short, i) => {
    assert.ok(short.command, `Short ${i} should have command`);
    assert.ok(short.command.includes('claw_shorts.py'), 
      `Short ${i} command should include claw_shorts.py`);
  });
});

test('generatePlanJSON is valid JSON', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  const jsonStr = JSON.stringify(plan);
  const parsed = JSON.parse(jsonStr);
  assert.deepStrictEqual(parsed, plan);
});

test('generatePlanJSON generated_at is ISO timestamp', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  const date = new Date(plan.generated_at);
  assert.ok(!isNaN(date.getTime()), 'generated_at should be valid date');
});

test('generatePlanJSON suggested_hashtags includes shorts', () => {
  const info = extractProjectInfo(SAMPLE_README);
  const hooks = generateHooks(info);
  const shots = generateShotList(info, hooks[0].text);
  const plan = generatePlanJSON(info, hooks[0].text, shots);
  
  assert.ok(plan.suggested_hashtags.includes('#shorts'));
});

console.log(`\nResults: ${passed} passed, ${failed} failed`);
process.exit(failed > 0 ? 1 : 0);
