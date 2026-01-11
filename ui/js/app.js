/**
 * Memory Graph - Main Application
 * Modern UI inspired by Sigma.js Demo
 */

// Global state
let graph = null;
let renderer = null;
let editor = null;
let selectedNode = null;
let showEdgeLabels = true;
let filters = {
  entityTypes: {},
  relationTypes: {}
};

// Initialize application
async function init() {
  try {
    // Load data
    editor = new DataEditor();
    await editor.loadFromURL('../memory.jsonl');

    // Initialize filters (all enabled by default)
    initFilters();

    // Build graph
    rebuildGraph();

    // Update UI
    updateStats();
    buildLegend();
    populateEntitySelect();
    updateSearchSuggestions();

    // Setup event handlers
    setupEventHandlers();
    setupGraphEvents();
    setupSearch();

    // Update graph subtitle
    document.getElementById('graph-subtitle').textContent =
      `${editor.entities.length} entities, ${editor.relations.length} relations`;

    console.log('Memory Graph initialized');
  } catch (error) {
    console.error('Failed to initialize:', error);
    showToast('Failed to load graph data', 'error');
  }
}

// Initialize filters from data
function initFilters() {
  // Entity types
  const entityTypes = [...new Set(editor.entities.map(e => e.entityType))].sort();
  entityTypes.forEach(type => {
    filters.entityTypes[type] = true;
  });

  // Relation types
  const relationTypes = [...new Set(editor.relations.map(r => r.relationType))].sort();
  relationTypes.forEach(type => {
    filters.relationTypes[type] = true;
  });

  // Build filter UI
  buildFilterUI();
}

// Build filter checkboxes
function buildFilterUI() {
  // Entity type filters
  const typeFiltersEl = document.getElementById('type-filters');
  typeFiltersEl.innerHTML = '';

  const entityTypes = Object.keys(filters.entityTypes).sort();
  entityTypes.forEach(type => {
    const count = editor.entities.filter(e => e.entityType === type).length;
    const color = GraphModule.colorByType(type);

    const item = document.createElement('label');
    item.className = 'filter-item';
    item.innerHTML = `
      <input type="checkbox" ${filters.entityTypes[type] ? 'checked' : ''} data-type="${type}">
      <span class="filter-dot" style="background:${color}"></span>
      <span class="filter-label">${type}</span>
      <span class="filter-count">${count}</span>
    `;
    typeFiltersEl.appendChild(item);
  });

  // Relation type filters
  const relationFiltersEl = document.getElementById('relation-filters');
  relationFiltersEl.innerHTML = '';

  const relationTypes = Object.keys(filters.relationTypes).sort();
  relationTypes.forEach(type => {
    const count = editor.relations.filter(r => r.relationType === type).length;

    const item = document.createElement('label');
    item.className = 'filter-item';
    item.innerHTML = `
      <input type="checkbox" ${filters.relationTypes[type] ? 'checked' : ''} data-relation="${type}">
      <span class="filter-dot" style="background:#f59e0b"></span>
      <span class="filter-label">${type}</span>
      <span class="filter-count">${count}</span>
    `;
    relationFiltersEl.appendChild(item);
  });
}

// Rebuild graph with current data
function rebuildGraph() {
  graph = GraphModule.buildGraph(editor.entities, editor.relations, false);

  if (renderer) {
    renderer.kill();
  }

  renderer = new Sigma(graph, document.getElementById("sigma-container"), {
    renderEdgeLabels: showEdgeLabels,
    labelDensity: 0.1,
    labelGridCellSize: 120,
    labelRenderedSizeThreshold: 8,
    defaultEdgeColor: '#475569',
    defaultEdgeType: 'arrow',
    edgeArrowPosition: 'target',
    edgeArrowSize: 4,
    labelColor: { color: '#e5e7eb' },
    edgeLabelColor: { color: '#f59e0b' },
    edgeLabelSize: 10,
    minCameraRatio: 0.05,
    maxCameraRatio: 10,
    defaultDrawNodeLabel: drawLabel,
    defaultDrawNodeHover: drawHover
  });

  // Apply filters
  applyFilters();

  setupGraphEvents();
}

// Custom label renderer (dark background, light text)
function drawLabel(context, data, settings) {
  if (!data.label) return;

  const size = settings.labelSize || 12;
  const font = settings.labelFont || 'Inter, sans-serif';
  const weight = settings.labelWeight || 'normal';

  context.font = `${weight} ${size}px ${font}`;
  const textWidth = context.measureText(data.label).width;
  const padding = 4;

  // Background
  context.fillStyle = 'rgba(15, 23, 42, 0.9)';
  context.fillRect(
    data.x + data.size + 2,
    data.y - size / 2 - padding,
    textWidth + padding * 2,
    size + padding * 2
  );

  // Text
  context.fillStyle = '#e5e7eb';
  context.fillText(data.label, data.x + data.size + 2 + padding, data.y + size / 3);
}

// Custom hover renderer (shows more info)
function drawHover(context, data, settings) {
  const size = settings.labelSize || 14;
  const font = settings.labelFont || 'Inter, sans-serif';

  // Draw larger node
  context.beginPath();
  context.arc(data.x, data.y, data.size + 4, 0, Math.PI * 2);
  context.fillStyle = data.color;
  context.fill();
  context.strokeStyle = '#f8fafc';
  context.lineWidth = 2;
  context.stroke();

  // Draw label with background
  if (data.label) {
    context.font = `bold ${size}px ${font}`;
    const textWidth = context.measureText(data.label).width;
    const padding = 8;
    const x = data.x + data.size + 6;
    const y = data.y - size / 2 - padding;

    // Background
    context.fillStyle = '#1e293b';
    context.strokeStyle = '#475569';
    context.lineWidth = 1;
    context.beginPath();
    context.roundRect(x, y, textWidth + padding * 2, size + padding * 2, 6);
    context.fill();
    context.stroke();

    // Text
    context.fillStyle = '#f8fafc';
    context.fillText(data.label, x + padding, data.y + size / 3);
  }
}

// Apply filters to graph
function applyFilters() {
  if (!graph || !renderer) return;

  graph.forEachNode((node, attrs) => {
    if (attrs.nodeType === 'entity') {
      const entityType = attrs.entityType || 'Unknown';
      const hidden = !filters.entityTypes[entityType];
      graph.setNodeAttribute(node, 'hidden', hidden);
    }
  });

  graph.forEachEdge((edge, attrs) => {
    const relationType = attrs.label || 'unknown';
    const hidden = !filters.relationTypes[relationType];
    graph.setEdgeAttribute(edge, 'hidden', hidden);
  });

  renderer.refresh();
}

// Setup graph interaction events
function setupGraphEvents() {
  if (!renderer) return;

  const tooltip = document.getElementById('tooltip');
  const tooltipTitle = document.getElementById('tooltip-title');
  const tooltipType = document.getElementById('tooltip-type');
  const tooltipObs = document.getElementById('tooltip-observations');

  // Hover - show tooltip
  renderer.on("enterNode", ({ node }) => {
    if (selectedNode === node) return;

    const attrs = graph.getNodeAttributes(node);
    tooltipTitle.textContent = attrs.fullName || attrs.label;
    tooltipType.textContent = attrs.entityType || '';

    tooltipObs.innerHTML = '';
    if (attrs.observations && attrs.observations.length > 0) {
      attrs.observations.slice(0, 3).forEach(obs => {
        const li = document.createElement('li');
        li.textContent = obs.length > 60 ? obs.substring(0, 60) + '...' : obs;
        tooltipObs.appendChild(li);
      });
    }

    tooltip.style.display = 'block';
  });

  renderer.on("leaveNode", () => {
    tooltip.style.display = 'none';
  });

  // Move tooltip with mouse
  renderer.getMouseCaptor().on("mousemove", (e) => {
    tooltip.style.left = e.x + 15 + 'px';
    tooltip.style.top = e.y + 15 + 'px';
  });

  // Click - select node and show details
  renderer.on("clickNode", ({ node }) => {
    const attrs = graph.getNodeAttributes(node);
    if (attrs.nodeType === 'entity') {
      selectedNode = node;
      showNodeDetail(attrs.fullName || attrs.label, attrs);
      highlightConnections(node);

      // Update entity select
      document.getElementById('entity-select').value = attrs.fullName || attrs.label;
      document.getElementById('run-inference').disabled = false;
    }
  });

  // Click stage - deselect
  renderer.on("clickStage", () => {
    closeNodeDetail();
  });
}

// Show node detail in right sidebar
function showNodeDetail(entityName, attrs) {
  document.getElementById('detail-title').textContent = entityName;
  document.getElementById('detail-type').textContent = attrs.entityType || 'Unknown';
  document.getElementById('detail-dot').style.background = attrs.color || '#3b82f6';

  // Observations
  const obsList = document.getElementById('detail-observations');
  obsList.innerHTML = '';
  if (attrs.observations && attrs.observations.length > 0) {
    attrs.observations.forEach(obs => {
      const li = document.createElement('li');
      li.textContent = obs;
      obsList.appendChild(li);
    });
  } else {
    obsList.innerHTML = '<li class="empty">No observations</li>';
  }

  // Connections
  const connectionsEl = document.getElementById('detail-connections');
  connectionsEl.innerHTML = '';

  const outgoing = editor.relations.filter(r => r.from === entityName);
  const incoming = editor.relations.filter(r => r.to === entityName);

  if (outgoing.length === 0 && incoming.length === 0) {
    connectionsEl.innerHTML = '<p class="empty">No connections</p>';
  } else {
    outgoing.forEach(rel => {
      const item = document.createElement('div');
      item.className = 'connection-item';
      item.innerHTML = `
        <span class="arrow">→</span>
        <span class="relation">${rel.relationType}</span>
        <span class="name">${rel.to}</span>
      `;
      item.onclick = () => focusOnNode(rel.to);
      connectionsEl.appendChild(item);
    });

    incoming.forEach(rel => {
      const item = document.createElement('div');
      item.className = 'connection-item';
      item.innerHTML = `
        <span class="arrow">←</span>
        <span class="relation">${rel.relationType}</span>
        <span class="name">${rel.from}</span>
      `;
      item.onclick = () => focusOnNode(rel.from);
      connectionsEl.appendChild(item);
    });
  }

  // Show right sidebar if collapsed
  document.getElementById('sidebar-right').classList.remove('collapsed');
}

// Close node detail
function closeNodeDetail() {
  selectedNode = null;
  document.getElementById('detail-title').textContent = 'No Selection';
  document.getElementById('detail-type').textContent = 'Click a node to view details';
  document.getElementById('detail-observations').innerHTML = '<li class="empty">No observations</li>';
  document.getElementById('detail-connections').innerHTML = '<p class="empty">No connections</p>';

  // Reset graph highlights
  if (renderer) {
    renderer.setSetting('nodeReducer', null);
    renderer.setSetting('edgeReducer', null);
  }
}

// Focus camera on a node
function focusOnNode(entityName) {
  const nodeId = `entity:${entityName}`;
  if (graph.hasNode(nodeId)) {
    const nodeDisplayData = renderer.getNodeDisplayData(nodeId);
    if (nodeDisplayData) {
      renderer.getCamera().animate(
        { x: nodeDisplayData.x, y: nodeDisplayData.y, ratio: 0.5 },
        { duration: 400 }
      );

      setTimeout(() => {
        const attrs = graph.getNodeAttributes(nodeId);
        selectedNode = nodeId;
        showNodeDetail(entityName, attrs);
        highlightConnections(nodeId);
      }, 200);
    }
  }
}

// Highlight connections for selected node
function highlightConnections(nodeId) {
  const NODE_FADE = '#334155';
  const EDGE_FADE = '#1e293b';

  renderer.setSetting('nodeReducer', (node, data) => {
    if (node === nodeId) {
      return { ...data, zIndex: 2 };
    }
    if (graph.hasEdge(node, nodeId) || graph.hasEdge(nodeId, node)) {
      return { ...data, zIndex: 1 };
    }
    return { ...data, color: NODE_FADE, label: '', zIndex: 0 };
  });

  renderer.setSetting('edgeReducer', (edge, data) => {
    if (graph.hasExtremity(edge, nodeId)) {
      return { ...data, size: 2, zIndex: 1 };
    }
    return { ...data, color: EDGE_FADE, hidden: true };
  });
}

// Update statistics
function updateStats() {
  document.getElementById('stat-entities').textContent = editor.entities.length;
  document.getElementById('stat-relations').textContent = editor.relations.length;
}

// Build legend
function buildLegend() {
  const legendGrid = document.getElementById('legend-grid');
  const types = [...new Set(editor.entities.map(e => e.entityType))].sort();

  legendGrid.innerHTML = types.map(type => `
    <div class="legend-item">
      <span class="legend-dot" style="background:${GraphModule.colorByType(type)}"></span>
      <span>${type}</span>
    </div>
  `).join('');
}

// Populate entity select dropdown
function populateEntitySelect() {
  const select = document.getElementById('entity-select');
  select.innerHTML = '<option value="">Click a node or select...</option>';

  editor.entities.forEach(e => {
    const opt = document.createElement('option');
    opt.value = e.name;
    opt.textContent = e.name;
    select.appendChild(opt);
  });
}

// Update search suggestions
function updateSearchSuggestions() {
  // Now handled by live search
}

// Setup search functionality
function setupSearch() {
  const searchInput = document.getElementById('graph-search');
  const searchResults = document.getElementById('search-results');
  let selectedIndex = -1;

  searchInput.addEventListener('input', () => {
    const query = searchInput.value.trim().toLowerCase();

    if (query.length < 1) {
      searchResults.classList.remove('active');
      return;
    }

    // Filter entities
    const matches = editor.entities.filter(e =>
      e.name.toLowerCase().includes(query) ||
      e.entityType.toLowerCase().includes(query)
    ).slice(0, 10);

    if (matches.length === 0) {
      searchResults.classList.remove('active');
      return;
    }

    // Build results with highlighted text
    searchResults.innerHTML = matches.map((e, i) => {
      const name = e.name.replace(
        new RegExp(`(${query})`, 'gi'),
        '<mark>$1</mark>'
      );
      const color = GraphModule.colorByType(e.entityType);
      return `
        <div class="search-result-item ${i === 0 ? 'selected' : ''}" data-name="${e.name}">
          <span class="dot" style="background:${color}"></span>
          <span class="name">${name}</span>
          <span class="type">${e.entityType}</span>
        </div>
      `;
    }).join('');

    searchResults.classList.add('active');
    selectedIndex = 0;
  });

  // Keyboard navigation
  searchInput.addEventListener('keydown', (e) => {
    const items = searchResults.querySelectorAll('.search-result-item');

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, items.length - 1);
      updateSelection(items);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      updateSelection(items);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (items[selectedIndex]) {
        selectSearchResult(items[selectedIndex].dataset.name);
      }
    } else if (e.key === 'Escape') {
      searchResults.classList.remove('active');
      searchInput.blur();
    }
  });

  function updateSelection(items) {
    items.forEach((item, i) => {
      item.classList.toggle('selected', i === selectedIndex);
    });
  }

  // Click on result
  searchResults.addEventListener('click', (e) => {
    const item = e.target.closest('.search-result-item');
    if (item) {
      selectSearchResult(item.dataset.name);
    }
  });

  function selectSearchResult(entityName) {
    searchInput.value = entityName;
    searchResults.classList.remove('active');
    focusOnNode(entityName);
  }

  // Close on click outside
  document.addEventListener('click', (e) => {
    if (!e.target.closest('.search-wrapper')) {
      searchResults.classList.remove('active');
    }
  });
}

// Run inference
function runInferenceHandler() {
  const entityName = document.getElementById('entity-select').value;
  const maxDepth = parseInt(document.getElementById('max-depth').value);
  const minConf = parseInt(document.getElementById('min-confidence').value) / 100;

  if (!entityName) {
    showToast('Please select an entity', 'error');
    return;
  }

  const btn = document.getElementById('run-inference');
  btn.disabled = true;
  btn.textContent = '⏳ Running...';

  setTimeout(() => {
    const results = InferenceModule.runInference(entityName, editor.relations, maxDepth, minConf);
    displayInferenceResults(results);

    btn.disabled = false;
    btn.textContent = '🧪 Run Inference';

    showToast(`Found ${results.length} inferred relations`, 'success');
  }, 100);
}

// Display inference results
function displayInferenceResults(results) {
  const container = document.getElementById('inference-results');

  if (results.length === 0) {
    container.innerHTML = `
      <div class="result-empty">
        <div class="icon">🔍</div>
        <p>No inferred relations found</p>
      </div>
    `;
    return;
  }

  container.innerHTML = results.map(r => {
    const confClass = r.confidence >= 0.7 ? '' : r.confidence >= 0.5 ? 'low' : 'very-low';
    return `
      <div class="result-item">
        <div class="path">${InferenceModule.formatPath(r.path)}</div>
        <div class="inferred">
          <span>→ ${r.target}</span>
          <span class="confidence ${confClass}">${(r.confidence * 100).toFixed(0)}%</span>
        </div>
      </div>
    `;
  }).join('');
}

// Setup all event handlers
function setupEventHandlers() {
  // Sidebar toggles
  document.getElementById('toggle-left').onclick = () => {
    document.getElementById('sidebar-left').classList.toggle('collapsed');
  };

  document.getElementById('toggle-right').onclick = () => {
    document.getElementById('sidebar-right').classList.toggle('collapsed');
  };

  // Panel toggles
  document.querySelectorAll('.panel-header').forEach(header => {
    header.onclick = () => {
      header.parentElement.classList.toggle('collapsed');
    };
  });

  // Filter checkboxes - Entity types
  document.getElementById('type-filters').addEventListener('change', (e) => {
    if (e.target.type === 'checkbox') {
      const type = e.target.dataset.type;
      filters.entityTypes[type] = e.target.checked;
      applyFilters();
    }
  });

  // Filter checkboxes - Relation types
  document.getElementById('relation-filters').addEventListener('change', (e) => {
    if (e.target.type === 'checkbox') {
      const type = e.target.dataset.relation;
      filters.relationTypes[type] = e.target.checked;
      applyFilters();
    }
  });

  // Graph controls
  document.getElementById('btn-zoom-in').onclick = () => {
    renderer.getCamera().animatedZoom({ duration: 300 });
  };

  document.getElementById('btn-zoom-out').onclick = () => {
    renderer.getCamera().animatedUnzoom({ duration: 300 });
  };

  document.getElementById('btn-reset').onclick = () => {
    renderer.getCamera().animatedReset({ duration: 300 });
    closeNodeDetail();
  };

  document.getElementById('btn-fullscreen').onclick = () => {
    const container = document.getElementById('graph-container');
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      container.requestFullscreen();
    }
  };

  document.getElementById('btn-toggle-labels').onclick = (e) => {
    showEdgeLabels = !showEdgeLabels;
    renderer.setSetting('renderEdgeLabels', showEdgeLabels);
    e.target.closest('.control-btn').classList.toggle('active', showEdgeLabels);
  };

  // Inference controls
  document.getElementById('entity-select').onchange = function() {
    document.getElementById('run-inference').disabled = !this.value;
    if (this.value) {
      focusOnNode(this.value);
    }
  };

  document.getElementById('max-depth').oninput = function() {
    document.getElementById('depth-value').textContent = this.value;
  };

  document.getElementById('min-confidence').oninput = function() {
    document.getElementById('confidence-value').textContent = (this.value / 100).toFixed(2);
  };

  document.getElementById('run-inference').onclick = runInferenceHandler;

  // Node detail actions
  document.getElementById('close-detail').onclick = closeNodeDetail;

  document.getElementById('btn-edit-node').onclick = () => {
    if (selectedNode) {
      const entityName = graph.getNodeAttribute(selectedNode, 'fullName');
      editEntity(entityName);
    }
  };

  document.getElementById('btn-delete-node').onclick = () => {
    if (selectedNode) {
      const entityName = graph.getNodeAttribute(selectedNode, 'fullName');
      deleteEntity(entityName);
    }
  };

  // Export
  document.getElementById('btn-export').onclick = () => {
    editor.downloadJSONL('memory-export.jsonl');
    showToast('Data exported', 'success');
  };

  // Modal close on overlay click
  document.querySelectorAll('.modal-overlay').forEach(overlay => {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        overlay.classList.remove('active');
      }
    });
  });
}

// Entity CRUD
function showAddEntityModal() {
  document.getElementById('entity-modal-title').textContent = '➕ Add Entity';
  document.getElementById('entity-name').value = '';
  document.getElementById('entity-type').value = '';
  document.getElementById('entity-observations').value = '';
  document.getElementById('entity-name').dataset.editMode = '';
  document.getElementById('entity-modal').classList.add('active');
}

function editEntity(entityName) {
  const entity = editor.entities.find(e => e.name === entityName);
  if (!entity) return;

  document.getElementById('entity-modal-title').textContent = '✏️ Edit Entity';
  document.getElementById('entity-name').value = entity.name;
  document.getElementById('entity-type').value = entity.entityType;
  document.getElementById('entity-observations').value = (entity.observations || []).join('\n');
  document.getElementById('entity-name').dataset.editMode = entityName;
  document.getElementById('entity-modal').classList.add('active');
}

function saveEntity() {
  const name = document.getElementById('entity-name').value.trim();
  const type = document.getElementById('entity-type').value.trim();
  const obsText = document.getElementById('entity-observations').value.trim();
  const editMode = document.getElementById('entity-name').dataset.editMode;

  if (!name || !type) {
    showToast('Name and Type are required', 'error');
    return;
  }

  const observations = obsText ? obsText.split('\n').filter(o => o.trim()) : [];

  if (editMode) {
    editor.updateEntity(editMode, { name, entityType: type, observations });
    showToast('Entity updated', 'success');
  } else {
    editor.addEntity({ name, entityType: type, observations });
    showToast('Entity created', 'success');
  }

  document.getElementById('entity-modal').classList.remove('active');

  // Refresh
  initFilters();
  rebuildGraph();
  updateStats();
  buildLegend();
  populateEntitySelect();
  updateSearchSuggestions();
}

function deleteEntity(entityName) {
  if (!confirm(`Delete entity "${entityName}"?`)) return;

  editor.deleteEntity(entityName);
  showToast('Entity deleted', 'success');
  closeNodeDetail();

  // Refresh
  initFilters();
  rebuildGraph();
  updateStats();
  buildLegend();
  populateEntitySelect();
  updateSearchSuggestions();
}

// Relation CRUD
function showAddRelationModal() {
  const fromSelect = document.getElementById('rel-from');
  const toSelect = document.getElementById('rel-to');

  fromSelect.innerHTML = '<option value="">Select entity...</option>';
  toSelect.innerHTML = '<option value="">Select entity...</option>';

  editor.entities.forEach(e => {
    fromSelect.innerHTML += `<option value="${e.name}">${e.name}</option>`;
    toSelect.innerHTML += `<option value="${e.name}">${e.name}</option>`;
  });

  document.getElementById('rel-type').value = '';
  document.getElementById('relation-modal').classList.add('active');
}

function saveRelation() {
  const from = document.getElementById('rel-from').value;
  const to = document.getElementById('rel-to').value;
  const type = document.getElementById('rel-type').value.trim();

  if (!from || !to || !type) {
    showToast('All fields are required', 'error');
    return;
  }

  editor.addRelation({ from, to, relationType: type });
  document.getElementById('relation-modal').classList.remove('active');
  showToast('Relation created', 'success');

  // Refresh
  initFilters();
  rebuildGraph();
  updateStats();
}

function deleteRelation(from, to, type) {
  if (!confirm(`Delete relation "${from}" → "${to}"?`)) return;

  editor.deleteRelation(from, to, type);
  showToast('Relation deleted', 'success');

  // Refresh
  initFilters();
  rebuildGraph();
  updateStats();
}

// Toast notification
function showToast(message, type = 'info') {
  const container = document.getElementById('toast-container');
  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.textContent = message;
  container.appendChild(toast);

  setTimeout(() => {
    toast.remove();
  }, 3000);
}

// ===================================
// Event Sourcing Stats
// ===================================

// Event Sourcing stats configuration
const ES_STATS_CONFIG = {
  // MCP server endpoint (update if needed)
  endpoint: null, // Will use mock data if null
  refreshInterval: 30000 // 30 seconds
};

// Fetch Event Sourcing stats from MCP server
async function fetchEventSourcingStats() {
  // For now, we'll parse stats from the JSONL file structure
  // In production, this would call the MCP server's get_stats tool
  try {
    const response = await fetch('../memory.jsonl');
    if (!response.ok) throw new Error('Failed to fetch data');

    const text = await response.text();
    const lines = text.trim().split('\n').filter(l => l.trim());

    // Count events by type
    const eventsByType = {
      'EntityCreated': 0,
      'RelationCreated': 0,
      'ObservationAdded': 0,
      'EntityDeleted': 0,
      'RelationDeleted': 0,
      'ObservationDeleted': 0
    };

    let totalEvents = 0;
    let lastEventId = 0;

    lines.forEach(line => {
      try {
        const data = JSON.parse(line);
        if (data.event_type) {
          // Event format
          totalEvents++;
          const eventType = data.event_type;
          if (eventsByType[eventType] !== undefined) {
            eventsByType[eventType]++;
          }
          if (data.event_id && data.event_id > lastEventId) {
            lastEventId = data.event_id;
          }
        } else if (data.type === 'entity' || data.type === 'relation') {
          // Legacy format - count as entities/relations
          if (data.type === 'entity') {
            eventsByType['EntityCreated']++;
          } else {
            eventsByType['RelationCreated']++;
          }
          totalEvents++;
        }
      } catch (e) {
        // Skip invalid lines
      }
    });

    // Get file size
    const fileSize = new Blob([text]).size;

    return {
      active_event_count: totalEvents,
      archived_event_count: 0,
      active_log_size: fileSize,
      archive_size: 0,
      snapshot_size: 0,
      archive_file_count: 0,
      events_by_type: eventsByType,
      last_event_id: lastEventId,
      last_snapshot_event_id: 0,
      events_since_snapshot: totalEvents,
      enabled: true
    };
  } catch (error) {
    console.error('Failed to fetch ES stats:', error);
    return null;
  }
}

// Format bytes to human readable
function formatBytes(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Update Event Sourcing stats UI
async function updateEventSourcingStats() {
  const stats = await fetchEventSourcingStats();

  const statusEl = document.getElementById('es-status');
  const statusTextEl = statusEl.querySelector('.es-status-text');

  if (!stats) {
    // Error state
    statusEl.className = 'es-status error';
    statusTextEl.textContent = 'Failed to fetch stats';
    document.getElementById('es-total-events').textContent = '-';
    document.getElementById('es-last-snapshot').textContent = '-';
    document.getElementById('es-archive-count').textContent = '-';
    document.getElementById('es-total-size').textContent = '-';
    return;
  }

  // Update stats values
  const totalEvents = stats.active_event_count + stats.archived_event_count;
  document.getElementById('es-total-events').textContent = totalEvents.toLocaleString();

  if (stats.last_snapshot_event_id > 0) {
    document.getElementById('es-last-snapshot').textContent = `#${stats.last_snapshot_event_id}`;
  } else {
    document.getElementById('es-last-snapshot').textContent = 'None';
  }

  document.getElementById('es-archive-count').textContent = stats.archive_file_count.toString();

  const totalSize = stats.active_log_size + stats.archive_size + stats.snapshot_size;
  document.getElementById('es-total-size').textContent = formatBytes(totalSize);

  // Update events breakdown
  const breakdownItemsEl = document.getElementById('es-breakdown-items');
  const eventTypes = Object.entries(stats.events_by_type || {}).filter(([_, count]) => count > 0);

  if (eventTypes.length > 0) {
    breakdownItemsEl.innerHTML = eventTypes.map(([type, count]) => {
      // Shorten event type names
      const shortType = type.replace('Created', '+').replace('Deleted', '-').replace('Added', '+');
      return `<span class="es-breakdown-item"><span class="type">${shortType}</span><span class="count">${count}</span></span>`;
    }).join('');
  } else {
    breakdownItemsEl.innerHTML = '<span class="es-breakdown-empty">No events recorded</span>';
  }

  // Update status
  if (stats.enabled) {
    statusEl.className = 'es-status active';
    statusTextEl.textContent = `Event Sourcing active • ${stats.events_since_snapshot} events since snapshot`;
  } else {
    statusEl.className = 'es-status inactive';
    statusTextEl.textContent = 'Event Sourcing disabled';
  }
}

// Setup Event Sourcing stats refresh
function setupEventSourcingStats() {
  // Initial load
  updateEventSourcingStats();

  // Refresh button
  const refreshBtn = document.getElementById('btn-refresh-es-stats');
  if (refreshBtn) {
    refreshBtn.addEventListener('click', () => {
      refreshBtn.disabled = true;
      refreshBtn.textContent = '⏳ Loading...';
      updateEventSourcingStats().finally(() => {
        refreshBtn.disabled = false;
        refreshBtn.textContent = '🔄 Refresh Stats';
        showToast('Stats refreshed', 'success');
      });
    });
  }

  // Auto-refresh (optional)
  if (ES_STATS_CONFIG.refreshInterval > 0) {
    setInterval(updateEventSourcingStats, ES_STATS_CONFIG.refreshInterval);
  }
}

// Start app
document.addEventListener('DOMContentLoaded', () => {
  init();
  setupEventSourcingStats();
});

// Export for onclick handlers
window.showAddEntityModal = showAddEntityModal;
window.editEntity = editEntity;
window.saveEntity = saveEntity;
window.deleteEntity = deleteEntity;
window.showAddRelationModal = showAddRelationModal;
window.saveRelation = saveRelation;
window.deleteRelation = deleteRelation;
window.closeNodeDetail = closeNodeDetail;
window.focusOnNode = focusOnNode;
