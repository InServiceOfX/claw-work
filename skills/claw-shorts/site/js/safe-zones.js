/**
 * 9:16 Safe Zone Checker
 * 
 * Overlay caption danger zones for YouTube Shorts, TikTok, and Reels.
 * Drop an image to preview safe and unsafe areas for text placement.
 * 
 * MIT License - see repository LICENSE
 * 
 * Zone dimensions are approximate and derived from:
 * - YouTube Shorts: Title/channel overlay at bottom ~15% of frame
 *   Source: YouTube Shorts ad specs (2023), vertical video best practices
 * - TikTok: Username/sounds at bottom ~25%, like/share buttons on right ~15%
 *   Source: TikTok vertical ads best practices guide, creator resources
 * - Instagram Reels: Username/caption at bottom ~20%, buttons on right
 *   Source: Instagram Reels ad specifications, Meta Business Help Center
 * 
 * These zones vary slightly by device/update. Test on target devices.
 */

(function() {
  'use strict';

  const FRAME_WIDTH = 1080;
  const FRAME_HEIGHT = 1920;

  const SAFE_ZONES = {
    youtube: {
      name: 'YouTube Shorts',
      color: 'rgba(255, 0, 0, 0.35)',
      borderColor: '#ff0000',
      zones: [
        { top: 0, height: 100, left: 0, right: 0, label: 'Status bar' },
        { bottom: 0, height: 288, left: 0, right: 0, label: 'Title/Subscribe' }
      ],
      safeArea: {
        top: 100,
        bottom: 288,
        left: 48,
        right: 48
      }
    },
    tiktok: {
      name: 'TikTok',
      color: 'rgba(0, 255, 255, 0.35)',
      borderColor: '#00e5e5',
      zones: [
        { top: 0, height: 120, left: 0, right: 0, label: 'Status bar' },
        { bottom: 0, height: 480, left: 0, right: 0, label: 'Username/Caption' },
        { top: 300, bottom: 300, right: 0, width: 120, label: 'Action buttons' }
      ],
      safeArea: {
        top: 120,
        bottom: 480,
        left: 48,
        right: 150
      }
    },
    reels: {
      name: 'Instagram Reels',
      color: 'rgba(255, 0, 255, 0.35)',
      borderColor: '#ff00ff',
      zones: [
        { top: 0, height: 150, left: 0, right: 0, label: 'Status/Camera' },
        { bottom: 0, height: 384, left: 0, right: 0, label: 'Username/Audio' },
        { top: 400, bottom: 400, right: 0, width: 100, label: 'Actions' }
      ],
      safeArea: {
        top: 150,
        bottom: 384,
        left: 48,
        right: 130
      }
    }
  };

  const COMBINED_SAFE_AREA = {
    top: 150,
    bottom: 480,
    left: 48,
    right: 150
  };

  function calculateZoneRect(zone, canvasWidth, canvasHeight) {
    const scaleX = canvasWidth / FRAME_WIDTH;
    const scaleY = canvasHeight / FRAME_HEIGHT;
    
    let x = zone.left !== undefined ? zone.left * scaleX : 0;
    let y = zone.top !== undefined ? zone.top * scaleY : 0;
    let w = zone.width !== undefined ? zone.width * scaleX : canvasWidth;
    let h = zone.height !== undefined ? zone.height * scaleY : canvasHeight;
    
    if (zone.right !== undefined && zone.width === undefined) {
      w = canvasWidth - x - (zone.right * scaleX);
    }
    if (zone.right !== undefined && zone.width !== undefined) {
      x = canvasWidth - zone.width * scaleX - zone.right * scaleX;
      w = zone.width * scaleX;
    }
    if (zone.bottom !== undefined && zone.height === undefined) {
      h = canvasHeight - y - (zone.bottom * scaleY);
    }
    if (zone.bottom !== undefined && zone.height !== undefined && zone.top === undefined) {
      y = canvasHeight - zone.height * scaleY;
      h = zone.height * scaleY;
    }
    
    return { x, y, w, h };
  }

  function drawZones(ctx, canvasWidth, canvasHeight, activePlatforms) {
    ctx.clearRect(0, 0, canvasWidth, canvasHeight);
    
    Object.entries(SAFE_ZONES).forEach(([platform, config]) => {
      if (!activePlatforms[platform]) return;
      
      config.zones.forEach(zone => {
        const rect = calculateZoneRect(zone, canvasWidth, canvasHeight);
        
        ctx.fillStyle = config.color;
        ctx.fillRect(rect.x, rect.y, rect.w, rect.h);
        
        ctx.strokeStyle = config.borderColor;
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 5]);
        ctx.strokeRect(rect.x, rect.y, rect.w, rect.h);
        ctx.setLineDash([]);
      });
    });

    const allActive = Object.values(activePlatforms).every(v => v);
    if (allActive) {
      const scaleX = canvasWidth / FRAME_WIDTH;
      const scaleY = canvasHeight / FRAME_HEIGHT;
      
      const safeX = COMBINED_SAFE_AREA.left * scaleX;
      const safeY = COMBINED_SAFE_AREA.top * scaleY;
      const safeW = canvasWidth - (COMBINED_SAFE_AREA.left + COMBINED_SAFE_AREA.right) * scaleX;
      const safeH = canvasHeight - (COMBINED_SAFE_AREA.top + COMBINED_SAFE_AREA.bottom) * scaleY;
      
      ctx.strokeStyle = '#3fb950';
      ctx.lineWidth = 3;
      ctx.setLineDash([10, 5]);
      ctx.strokeRect(safeX, safeY, safeW, safeH);
      ctx.setLineDash([]);
      
      ctx.fillStyle = 'rgba(63, 185, 80, 0.1)';
      ctx.fillRect(safeX, safeY, safeW, safeH);
    }
  }

  function analyzeImageForText(imageData) {
    const warnings = [];
    return warnings;
  }

  function getSafeAreaInfo(activePlatforms) {
    const platforms = Object.entries(activePlatforms)
      .filter(([_, active]) => active)
      .map(([name]) => SAFE_ZONES[name].name);
    
    if (platforms.length === 0) {
      return null;
    }

    if (platforms.length === 3) {
      return {
        platforms: 'All platforms',
        topPx: COMBINED_SAFE_AREA.top,
        bottomPx: COMBINED_SAFE_AREA.bottom,
        leftPx: COMBINED_SAFE_AREA.left,
        rightPx: COMBINED_SAFE_AREA.right,
        safeHeight: FRAME_HEIGHT - COMBINED_SAFE_AREA.top - COMBINED_SAFE_AREA.bottom,
        safeWidth: FRAME_WIDTH - COMBINED_SAFE_AREA.left - COMBINED_SAFE_AREA.right
      };
    }

    let maxTop = 0, maxBottom = 0, maxLeft = 0, maxRight = 0;
    Object.entries(activePlatforms).forEach(([platform, active]) => {
      if (!active) return;
      const safe = SAFE_ZONES[platform].safeArea;
      maxTop = Math.max(maxTop, safe.top);
      maxBottom = Math.max(maxBottom, safe.bottom);
      maxLeft = Math.max(maxLeft, safe.left);
      maxRight = Math.max(maxRight, safe.right);
    });

    return {
      platforms: platforms.join(', '),
      topPx: maxTop,
      bottomPx: maxBottom,
      leftPx: maxLeft,
      rightPx: maxRight,
      safeHeight: FRAME_HEIGHT - maxTop - maxBottom,
      safeWidth: FRAME_WIDTH - maxLeft - maxRight
    };
  }

  function init() {
    const dropZone = document.getElementById('drop-zone');
    const fileInput = document.getElementById('file-input');
    const canvas = document.getElementById('preview-canvas');
    const overlayCanvas = document.getElementById('overlay-canvas');
    const previewContainer = document.getElementById('preview-container');
    const infoPanel = document.getElementById('zone-info');
    const platformToggles = document.querySelectorAll('.platform-toggle');

    if (!dropZone || !canvas) return;

    const ctx = canvas.getContext('2d');
    const overlayCtx = overlayCanvas.getContext('2d');
    let loadedImage = null;
    let activePlatforms = {
      youtube: true,
      tiktok: true,
      reels: true
    };

    function updateOverlay() {
      if (!loadedImage) return;
      drawZones(overlayCtx, overlayCanvas.width, overlayCanvas.height, activePlatforms);
      updateInfoPanel();
    }

    function updateInfoPanel() {
      const info = getSafeAreaInfo(activePlatforms);
      if (!info || !infoPanel) return;
      
      infoPanel.innerHTML = `
        <h4>Safe Zone (${info.platforms})</h4>
        <ul>
          <li>Top margin: <strong>${info.topPx}px</strong></li>
          <li>Bottom margin: <strong>${info.bottomPx}px</strong></li>
          <li>Left margin: <strong>${info.leftPx}px</strong></li>
          <li>Right margin: <strong>${info.rightPx}px</strong></li>
          <li>Safe area: <strong>${info.safeWidth} × ${info.safeHeight}px</strong></li>
        </ul>
        <p class="text-sm text-muted">
          Place all important text and elements within the green safe zone.
        </p>
      `;
    }

    function loadImage(file) {
      if (!file.type.startsWith('image/')) {
        alert('Please drop an image file');
        return;
      }

      const reader = new FileReader();
      reader.onload = (e) => {
        const img = new Image();
        img.onload = () => {
          loadedImage = img;
          
          const aspectRatio = 9 / 16;
          const containerWidth = previewContainer.clientWidth || 360;
          const displayWidth = Math.min(containerWidth, 360);
          const displayHeight = displayWidth / aspectRatio;
          
          canvas.width = displayWidth;
          canvas.height = displayHeight;
          overlayCanvas.width = displayWidth;
          overlayCanvas.height = displayHeight;
          
          ctx.fillStyle = '#000';
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          
          const imgAspect = img.width / img.height;
          const targetAspect = displayWidth / displayHeight;
          let drawWidth, drawHeight, drawX, drawY;
          
          if (imgAspect > targetAspect) {
            drawHeight = displayHeight;
            drawWidth = drawHeight * imgAspect;
            drawX = (displayWidth - drawWidth) / 2;
            drawY = 0;
          } else {
            drawWidth = displayWidth;
            drawHeight = drawWidth / imgAspect;
            drawX = 0;
            drawY = (displayHeight - drawHeight) / 2;
          }
          
          ctx.drawImage(img, drawX, drawY, drawWidth, drawHeight);
          
          dropZone.style.display = 'none';
          canvas.style.display = 'block';
          overlayCanvas.style.display = 'block';
          
          updateOverlay();
        };
        img.src = e.target.result;
      };
      reader.readAsDataURL(file);
    }

    dropZone.addEventListener('click', () => fileInput.click());
    
    fileInput.addEventListener('change', (e) => {
      if (e.target.files[0]) {
        loadImage(e.target.files[0]);
      }
    });

    dropZone.addEventListener('dragover', (e) => {
      e.preventDefault();
      dropZone.classList.add('dragover');
    });

    dropZone.addEventListener('dragleave', () => {
      dropZone.classList.remove('dragover');
    });

    dropZone.addEventListener('drop', (e) => {
      e.preventDefault();
      dropZone.classList.remove('dragover');
      if (e.dataTransfer.files[0]) {
        loadImage(e.dataTransfer.files[0]);
      }
    });

    platformToggles.forEach(toggle => {
      toggle.addEventListener('change', () => {
        const platform = toggle.dataset.platform;
        activePlatforms[platform] = toggle.checked;
        updateOverlay();
      });
    });

    document.getElementById('reset-btn')?.addEventListener('click', () => {
      loadedImage = null;
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      overlayCtx.clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);
      dropZone.style.display = 'flex';
      canvas.style.display = 'none';
      overlayCanvas.style.display = 'none';
    });

    updateInfoPanel();
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
      SAFE_ZONES,
      COMBINED_SAFE_AREA,
      FRAME_WIDTH,
      FRAME_HEIGHT,
      calculateZoneRect,
      getSafeAreaInfo
    };
  }
})();
