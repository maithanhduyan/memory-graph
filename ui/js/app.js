// Main application module

let editor = null;
let graph = null;
let renderer = null;
let showObservations = false;
let showEdgeLabels = true;
let inferredEdges = [];
let selectedNode = null;

// Initialize application
async function init() {
  editor = new DataEditor();

  // Load data
  try {
    await editor.loadFromURL('../memory.jsonl');
  } catch (e) {
    console.error('Failed to load memory.jsonl:', e);
    showToast('Failed to load data file', 'error');
    return;
  }

  // Set up data change handler
  editor.onDataChange = (entities, relations) => {
    rebuildGraph();
    updateStats();
    updateEntityList();
    updateRelationList();
    populateEntitySelect();
  };

  // Initial render
  rebuildGraph();
  updateStats();
  buildLegend();
  populateEntitySelect();
  updateEntityList();
  updateRelationList();
  setupEventListeners();
}

// Rebuild graph visualization
function rebuildGraph() {
  graph = GraphModule.buildGraph(editor.entities, editor.relations, showObservations);

  if (renderer) {
    renderer.kill();
  }

  renderer = new Sigma(graph, document.getElementById("container"), {
    renderEdgeLabels: showEdgeLabels,
    labelDensity: 0.12,
    labelGridCellSize: 150,
    labelRenderedSizeThreshold: 10,
    defaultEdgeColor: '#475569',
    labelColor: { color: '#e5e7eb' },
    edgeLabelColor: { color: '#f59e0b' },
    edgeLabelSize: 10,
    minCameraRatio: 0.05,
    maxCameraRatio: 10,
    // Hover label styles - dark background with light text
    labelHoverBgColor: '#1e293b',
    labelHoverColor: { color: '#f8fafc' },
    labelHoverShadow: 'none',
    labelHoverShadowColor: 'transparent',
    // Highlighted node label (when selected/hovered)
    hoverRenderer: (context, data, settings) => {
      const size = data.size + 4;
      const label = data.label;
      const fontSize = settings.labelSize || 14;

      // Draw larger node circle
      context.beginPath();
      context.arc(data.x, data.y, size, 0, Math.PI * 2);
      context.fillStyle = data.color;
      context.fill();
      context.strokeStyle = '#f8fafc';
      context.lineWidth = 2;
      context.stroke();

      // Draw label with dark background
      if (label) {
        context.font = `bold ${fontSize}px "Inter", sans-serif`;
        const textWidth = context.measureText(label).width;
        const padding = 6;
        const bgX = data.x + size + 4;
        const bgY = data.y - fontSize / 2 - padding;
        const bgWidth = textWidth + padding * 2;
        const bgHeight = fontSize + padding * 2;

        // Dark background
        context.fillStyle = '#1e293b';
        context.strokeStyle = '#475569';
        context.lineWidth = 1;
        context.beginPath();
        context.roundRect(bgX, bgY, bgWidth, bgHeight, 4);
        context.fill();
        context.stroke();

        // Light text
        context.fillStyle = '#f8fafc';
        context.fillText(label, bgX + padding, data.y + fontSize / 3);
      }
    }
  });

  setupGraphEvents();
}

// Setup graph interaction events
function setupGraphEvents() {
  const tooltip = document.getElementById('tooltip');
  const tooltipTitle = document.getElementById('tooltip-title');
  const tooltipType = document.getElementById('tooltip-type');
  const tooltipObs = document.getElementById('tooltip-observations');

  renderer.on("enterNode", ({ node }) => {
    // Don't show tooltip if detail panel is open for this node
    if (selectedNode === node) return;

    const attrs = graph.getNodeAttributes(node);
    tooltipTitle.textContent = attrs.fullName || attrs.label;
    tooltipType.innerHTML = `<span class="type-badge">${attrs.nodeType}</span> ${attrs.entityType || ''}`;
    tooltipObs.innerHTML = '';

    if (attrs.observations && attrs.observations.length > 0) {
      attrs.observations.slice(0, 3).forEach(obs => {
        const li = document.createElement('li');
        li.textContent = obs.length > 80 ? obs.substring(0, 80) + '...' : obs;
        tooltipObs.appendChild(li);
      });
      if (attrs.observations.length > 3) {
        const li = document.createElement('li');
        li.style.color = '#64748b';
        li.style.fontStyle = 'italic';
        li.textContent = `+ ${attrs.observations.length - 3} more... (click to view all)`;
        tooltipObs.appendChild(li);
      }
    } else if (attrs.fullText) {
      const li = document.createElement('li');
      li.textContent = attrs.fullText;
      tooltipObs.appendChild(li);
    }

    tooltip.style.display = 'block';
  });

  renderer.on("leaveNode", () => {
    tooltip.style.display = 'none';
  });

  renderer.on("clickNode", ({ node }) => {
    const attrs = graph.getNodeAttributes(node);
    if (attrs.nodeType === 'entity') {
      selectedNode = node;

      // Update entity select for inference
      document.getElementById('entity-select').value = attrs.fullName || attrs.label;
      document.getElementById('run-inference').disabled = false;

      // Highlight in entity list
      const listItems = document.querySelectorAll('#entity-list .list-item');
      listItems.forEach(item => {
        item.classList.toggle('selected', item.dataset.name === (attrs.fullName || attrs.label));
      });

      // Show node detail panel
      showNodeDetail(attrs.fullName || attrs.label, attrs);

      // Highlight connected edges
      highlightConnections(node);
    }
  });

  // Click on stage (background) to deselect
  renderer.on("clickStage", () => {
    closeNodeDetail();
    resetHighlights();
  });
}

// Show node detail panel
function showNodeDetail(entityName, attrs) {
  const panel = document.getElementById('node-detail-panel');
  const detailDot = document.getElementById('detail-dot');
  const detailTitle = document.getElementById('detail-title');
  const detailType = document.getElementById('detail-type');
  const detailObs = document.getElementById('detail-observations');
  const detailConns = document.getElementById('detail-connections');

  // Hide tooltip
  document.getElementById('tooltip').style.display = 'none';

  // Set header info
  detailDot.style.background = GraphModule.colorByType(attrs.entityType);
  detailTitle.textContent = entityName;
  detailType.textContent = attrs.entityType;

  // Set observations
  detailObs.innerHTML = '';
  if (attrs.observations && attrs.observations.length > 0) {
    attrs.observations.forEach(obs => {
      const li = document.createElement('li');
      li.textContent = obs;
      detailObs.appendChild(li);
    });
  } else {
    detailObs.innerHTML = '<li style="color: #64748b; font-style: italic;">No observations</li>';
  }

  // Set connections
  const connections = editor.getRelationsForEntity(entityName);
  detailConns.innerHTML = '';

  if (connections.outgoing.length === 0 && connections.incoming.length === 0) {
    detailConns.innerHTML = '<div style="color: #64748b; font-style: italic; padding: 8px;">No connections</div>';
  } else {
    // Outgoing connections
    if (connections.outgoing.length > 0) {
      const outGroup = document.createElement('div');
      outGroup.className = 'connection-group';
      outGroup.innerHTML = `
        <div class="connection-group-title">
          <span>→ Outgoing</span>
          <span class="count">${connections.outgoing.length}</span>
        </div>
      `;
      connections.outgoing.forEach(rel => {
        const item = document.createElement('div');
        item.className = 'connection-item';
        item.onclick = () => focusOnNode(rel.to);
        item.innerHTML = `
          <span class="direction outgoing">→</span>
          <div class="conn-info">
            <div class="conn-name" title="${rel.to}">${rel.to}</div>
            <div class="conn-type">${rel.relationType}</div>
          </div>
        `;
        outGroup.appendChild(item);
      });
      detailConns.appendChild(outGroup);
    }

    // Incoming connections
    if (connections.incoming.length > 0) {
      const inGroup = document.createElement('div');
      inGroup.className = 'connection-group';
      inGroup.innerHTML = `
        <div class="connection-group-title">
          <span>← Incoming</span>
          <span class="count">${connections.incoming.length}</span>
        </div>
      `;
      connections.incoming.forEach(rel => {
        const item = document.createElement('div');
        item.className = 'connection-item';
        item.onclick = () => focusOnNode(rel.from);
        item.innerHTML = `
          <span class="direction incoming">←</span>
          <div class="conn-info">
            <div class="conn-name" title="${rel.from}">${rel.from}</div>
            <div class="conn-type">${rel.relationType}</div>
          </div>
        `;
        inGroup.appendChild(item);
      });
      detailConns.appendChild(inGroup);
    }
  }

  panel.classList.add('active');
}

// Close node detail panel
function closeNodeDetail() {
  document.getElementById('node-detail-panel').classList.remove('active');
  selectedNode = null;
  resetHighlights();
}

// Focus camera on a node
function focusOnNode(entityName) {
  const nodeId = `entity:${entityName}`;
  if (graph.hasNode(nodeId)) {
    const attrs = graph.getNodeAttributes(nodeId);

    // Get current camera ratio, don't zoom in too much
    const currentRatio = renderer.getCamera().getState().ratio;
    const targetRatio = Math.min(currentRatio, 1.5); // Zoom in but not too close

    // Animate camera to node
    renderer.getCamera().animate({
      x: attrs.x,
      y: attrs.y,
      ratio: targetRatio
    }, { duration: 400 });

    // Select the node after animation
    setTimeout(() => {
      selectedNode = nodeId;
      showNodeDetail(entityName, attrs);
      highlightConnections(nodeId);
    }, 300);
  }
}

// Highlight connections for selected node
function highlightConnections(nodeId) {
  // Reset all node/edge colors first
  graph.forEachNode((node, attrs) => {
    if (node.startsWith('entity:') || node.startsWith('type:')) {
      graph.setNodeAttribute(node, 'highlighted', false);
    }
  });

  graph.forEachEdge((edge, attrs) => {
    graph.setEdgeAttribute(edge, 'highlighted', false);
  });

  // Highlight selected node
  graph.setNodeAttribute(nodeId, 'highlighted', true);

  // Highlight connected edges and nodes
  graph.forEachEdge(nodeId, (edge, attrs, source, target) => {
    graph.setEdgeAttribute(edge, 'highlighted', true);
    if (source !== nodeId) graph.setNodeAttribute(source, 'highlighted', true);
    if (target !== nodeId) graph.setNodeAttribute(target, 'highlighted', true);
  });

  // Apply visual changes via reducers
  renderer.setSetting('nodeReducer', (node, data) => {
    const highlighted = graph.getNodeAttribute(node, 'highlighted');
    if (selectedNode && !highlighted && node !== selectedNode) {
      return { ...data, color: '#334155', label: null };
    }
    return data;
  });

  renderer.setSetting('edgeReducer', (edge, data) => {
    const highlighted = graph.getEdgeAttribute(edge, 'highlighted');
    if (selectedNode && !highlighted) {
      return { ...data, color: '#1e293b', size: 0.5 };
    }
    if (highlighted) {
      return { ...data, color: '#22c55e', size: 3 };
    }
    return data;
  });

  renderer.refresh();
}

// Reset node/edge highlights
function resetHighlights() {
  renderer.setSetting('nodeReducer', null);
  renderer.setSetting('edgeReducer', null);
  renderer.refresh();
}

// Update statistics display
function updateStats() {
  const stats = GraphModule.calculateStats(editor.entities, editor.relations);
  document.getElementById('stat-entities').textContent = stats.entities;
  document.getElementById('stat-relations').textContent = stats.relations;
  document.getElementById('stat-types').textContent = stats.types;
  document.getElementById('stat-observations').textContent = stats.observations;
}

// Build legend
function buildLegend() {
  const legendGrid = document.getElementById('legend-grid');
  legendGrid.innerHTML = '';

  const types = editor.getEntityTypes();
  types.forEach(type => {
    const item = document.createElement('div');
    item.className = 'legend-item';
    item.innerHTML = `<span class="legend-dot" style="background:${GraphModule.colorByType(type)}"></span>${type}`;
    legendGrid.appendChild(item);
  });

  // Add special legend items
  const relItem = document.createElement('div');
  relItem.className = 'legend-item';
  relItem.innerHTML = `<span class="legend-dot" style="background:#f59e0b"></span>Relation`;
  legendGrid.appendChild(relItem);

  const inferItem = document.createElement('div');
  inferItem.className = 'legend-item';
  inferItem.innerHTML = `<span class="legend-dot" style="background:#8b5cf6"></span>Inferred`;
  legendGrid.appendChild(inferItem);
}

// Populate entity select dropdown
function populateEntitySelect() {
  const select = document.getElementById('entity-select');
  select.innerHTML = '<option value="">Select an entity...</option>';

  const groupedEntities = {};
  editor.entities.forEach(e => {
    if (!groupedEntities[e.entityType]) groupedEntities[e.entityType] = [];
    groupedEntities[e.entityType].push(e.name);
  });

  Object.entries(groupedEntities).sort().forEach(([type, names]) => {
    const optgroup = document.createElement('optgroup');
    optgroup.label = type;
    names.sort().forEach(name => {
      const option = document.createElement('option');
      option.value = name;
      option.textContent = name.length > 35 ? name.substring(0, 35) + '...' : name;
      optgroup.appendChild(option);
    });
    select.appendChild(optgroup);
  });

  // Also populate from/to selects in relation modal
  populateRelationSelects();
}

// Populate relation modal selects
function populateRelationSelects() {
  const fromSelect = document.getElementById('rel-from');
  const toSelect = document.getElementById('rel-to');

  if (!fromSelect || !toSelect) return;

  [fromSelect, toSelect].forEach(select => {
    const currentVal = select.value;
    select.innerHTML = '<option value="">Select entity...</option>';
    editor.entities.forEach(e => {
      const option = document.createElement('option');
      option.value = e.name;
      option.textContent = e.name.length > 30 ? e.name.substring(0, 30) + '...' : e.name;
      select.appendChild(option);
    });
    if (currentVal) select.value = currentVal;
  });
}

// Update entity list
function updateEntityList() {
  const list = document.getElementById('entity-list');
  const searchQuery = document.getElementById('entity-search')?.value?.toLowerCase() || '';

  const filtered = searchQuery
    ? editor.searchEntities(searchQuery)
    : editor.entities;

  list.innerHTML = filtered.map(e => `
    <div class="list-item" data-name="${e.name}">
      <span class="dot" style="background:${GraphModule.colorByType(e.entityType)}"></span>
      <span class="name" title="${e.name}">${e.name}</span>
      <span class="type">${e.entityType}</span>
      <span class="actions">
        <button class="edit" onclick="editEntity('${e.name.replace(/'/g, "\\'")}')">✎</button>
        <button class="delete" onclick="deleteEntity('${e.name.replace(/'/g, "\\'")}')">✕</button>
      </span>
    </div>
  `).join('');
}

// Update relation list
function updateRelationList() {
  const list = document.getElementById('relation-list');
  const searchQuery = document.getElementById('relation-search')?.value?.toLowerCase() || '';

  const filtered = searchQuery
    ? editor.relations.filter(r =>
        r.from.toLowerCase().includes(searchQuery) ||
        r.to.toLowerCase().includes(searchQuery) ||
        r.relationType.toLowerCase().includes(searchQuery))
    : editor.relations;

  list.innerHTML = filtered.map(r => `
    <div class="list-item" data-from="${r.from}" data-to="${r.to}" data-type="${r.relationType}">
      <span class="dot" style="background:#f59e0b"></span>
      <span class="name" title="${r.from} → ${r.to}">${r.from.substring(0, 15)}... → ${r.to.substring(0, 15)}...</span>
      <span class="type">${r.relationType}</span>
      <span class="actions">
        <button class="delete" onclick="deleteRelation('${r.from.replace(/'/g, "\\'")}', '${r.to.replace(/'/g, "\\'")}', '${r.relationType}')">✕</button>
      </span>
    </div>
  `).join('');
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
  btn.innerHTML = '<span class="spinner"></span><span>Running...</span>';

  setTimeout(() => {
    const results = InferenceModule.runInference(entityName, editor.relations, maxDepth, minConf);
    displayInferenceResults(results);
    highlightInferredEdges(results);

    btn.disabled = false;
    btn.innerHTML = '<span>🧪</span><span>Run Inference</span>';

    showToast(`Found ${results.length} inferred relations`, 'success');
  }, 300);
}

// Display inference results
function displayInferenceResults(results) {
  const container = document.getElementById('inference-results');

  if (results.length === 0) {
    container.innerHTML = `
      <div class="result-empty">
        <div class="icon">🤷</div>
        <div>No inferred relations found<br>Try adjusting parameters</div>
      </div>
    `;
    return;
  }

  container.innerHTML = results.map((r, i) => {
    const confPercent = (r.confidence * 100).toFixed(1);
    const confColor = InferenceModule.getConfidenceColor(r.confidence);

    return `
      <div class="inference-card" style="animation-delay: ${i * 30}ms">
        <span class="relation-type">${r.relationType}</span>
        <div class="path">${InferenceModule.formatPath(r.path, r.pathRelations)}</div>
        <div class="confidence">
          <span>Confidence:</span>
          <div class="confidence-bar">
            <div class="confidence-fill" style="width: ${confPercent}%; background: ${confColor}"></div>
          </div>
          <span class="confidence-value" style="color: ${confColor}">${confPercent}%</span>
        </div>
      </div>
    `;
  }).join('');
}

// Highlight inferred edges on graph
function highlightInferredEdges(results) {
  // Remove old inferred edges
  inferredEdges.forEach(edgeKey => {
    if (graph.hasEdge(edgeKey)) {
      graph.dropEdge(edgeKey);
    }
  });
  inferredEdges = [];

  // Add new inferred edges
  results.forEach(r => {
    const fromNode = `entity:${r.from}`;
    const toNode = `entity:${r.to}`;

    if (graph.hasNode(fromNode) && graph.hasNode(toNode)) {
      const edgeKey = `inferred:${r.from}->${r.to}:${r.relationType}`;
      if (!graph.hasEdge(edgeKey)) {
        graph.addEdgeWithKey(edgeKey, fromNode, toNode, {
          size: 3,
          color: "#8b5cf6",
          label: r.relationType,
          edgeType: 'inferred'
        });
        inferredEdges.push(edgeKey);
      }
    }
  });

  renderer.refresh();
}

// Entity CRUD handlers
function showAddEntityModal() {
  document.getElementById('entity-modal-title').textContent = '➕ Add Entity';
  document.getElementById('entity-name').value = '';
  document.getElementById('entity-type').value = '';
  document.getElementById('entity-observations').value = '';
  document.getElementById('entity-modal').dataset.mode = 'add';
  document.getElementById('entity-modal').classList.add('active');
}

function editEntity(name) {
  const entity = editor.entities.find(e => e.name === name);
  if (!entity) return;

  document.getElementById('entity-modal-title').textContent = '✏️ Edit Entity';
  document.getElementById('entity-name').value = entity.name;
  document.getElementById('entity-type').value = entity.entityType;
  document.getElementById('entity-observations').value = (entity.observations || []).join('\n');
  document.getElementById('entity-modal').dataset.mode = 'edit';
  document.getElementById('entity-modal').dataset.originalName = name;
  document.getElementById('entity-modal').classList.add('active');
}

function saveEntity() {
  const modal = document.getElementById('entity-modal');
  const mode = modal.dataset.mode;
  const name = document.getElementById('entity-name').value.trim();
  const type = document.getElementById('entity-type').value.trim();
  const observations = document.getElementById('entity-observations').value
    .split('\n')
    .map(o => o.trim())
    .filter(o => o);

  try {
    if (mode === 'add') {
      editor.addEntity({ name, entityType: type, observations });
      showToast(`Entity "${name}" created`, 'success');
    } else {
      const originalName = modal.dataset.originalName;
      editor.updateEntity(originalName, { name, entityType: type, observations });
      showToast(`Entity "${name}" updated`, 'success');
    }
    modal.classList.remove('active');
  } catch (e) {
    showToast(e.message, 'error');
  }
}

function deleteEntity(name) {
  if (!confirm(`Delete entity "${name}" and all its relations?`)) return;

  try {
    editor.deleteEntity(name);
    showToast(`Entity "${name}" deleted`, 'success');
  } catch (e) {
    showToast(e.message, 'error');
  }
}

// Relation CRUD handlers
function showAddRelationModal() {
  populateRelationSelects();
  document.getElementById('rel-from').value = '';
  document.getElementById('rel-to').value = '';
  document.getElementById('rel-type').value = '';
  document.getElementById('relation-modal').classList.add('active');
}

function saveRelation() {
  const from = document.getElementById('rel-from').value;
  const to = document.getElementById('rel-to').value;
  const relationType = document.getElementById('rel-type').value.trim();

  try {
    editor.addRelation({ from, to, relationType });
    showToast(`Relation created`, 'success');
    document.getElementById('relation-modal').classList.remove('active');
  } catch (e) {
    showToast(e.message, 'error');
  }
}

function deleteRelation(from, to, relationType) {
  if (!confirm(`Delete relation "${from}" → "${to}"?`)) return;

  try {
    editor.deleteRelation(from, to, relationType);
    showToast('Relation deleted', 'success');
  } catch (e) {
    showToast(e.message, 'error');
  }
}

// Setup event listeners
function setupEventListeners() {
  // Tabs
  document.querySelectorAll('.tab').forEach(tab => {
    tab.addEventListener('click', () => {
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
      document.querySelectorAll('.collapsed-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(tab.dataset.tab).classList.add('active');
      // Sync collapsed tabs
      document.querySelector(`.collapsed-tab[data-tab="${tab.dataset.tab}"]`)?.classList.add('active');
    });
  });

  // Collapsed tabs (when sidebar is collapsed)
  document.querySelectorAll('.collapsed-tab').forEach(tab => {
    tab.addEventListener('click', () => {
      const sidebar = document.getElementById('sidebar');
      // Expand sidebar when clicking collapsed tab
      sidebar.classList.remove('collapsed');
      document.getElementById('collapse-btn').textContent = '◀';
      // Switch to that tab
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
      document.querySelectorAll('.collapsed-tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      document.getElementById(tab.dataset.tab).classList.add('active');
      document.querySelector(`.tab[data-tab="${tab.dataset.tab}"]`)?.classList.add('active');
    });
  });

  // Sidebar collapse button
  document.getElementById('collapse-btn').addEventListener('click', () => {
    const sidebar = document.getElementById('sidebar');
    const btn = document.getElementById('collapse-btn');
    sidebar.classList.toggle('collapsed');
    btn.textContent = sidebar.classList.contains('collapsed') ? '▶' : '◀';
    // Refresh graph renderer after transition
    setTimeout(() => {
      if (renderer) renderer.refresh();
    }, 350);
  });

  // Collapsible panels
  document.querySelectorAll('.panel-header').forEach(header => {
    header.addEventListener('click', () => {
      const panel = header.parentElement;
      panel.classList.toggle('collapsed');
    });
  });

  // Toolbar buttons
  document.getElementById('btn-zoom-in').onclick = () => {
    renderer.getCamera().animatedZoom({ duration: 300 });
  };

  document.getElementById('btn-zoom-out').onclick = () => {
    renderer.getCamera().animatedUnzoom({ duration: 300 });
  };

  document.getElementById('btn-reset').onclick = () => {
    renderer.getCamera().animatedReset({ duration: 500 });
  };

  document.getElementById('btn-toggle-obs').onclick = function() {
    showObservations = !showObservations;
    this.textContent = showObservations ? '👁 Hide Obs' : '👁 Show Obs';
    this.classList.toggle('active', showObservations);
    rebuildGraph();
  };

  // Toggle edge labels
  document.getElementById('btn-toggle-labels').onclick = function() {
    showEdgeLabels = !showEdgeLabels;
    this.classList.toggle('active', showEdgeLabels);
    renderer.setSetting('renderEdgeLabels', showEdgeLabels);
    renderer.refresh();
  };

  // Inference controls
  document.getElementById('entity-select').onchange = () => {
    document.getElementById('run-inference').disabled = !document.getElementById('entity-select').value;
  };

  document.getElementById('max-depth').oninput = function() {
    document.getElementById('depth-value').textContent = this.value;
  };

  document.getElementById('min-confidence').oninput = function() {
    document.getElementById('confidence-value').textContent = (this.value / 100).toFixed(2);
  };

  document.getElementById('run-inference').onclick = runInferenceHandler;

  // Search
  document.getElementById('entity-search').oninput = updateEntityList;
  document.getElementById('relation-search').oninput = updateRelationList;

  // Click on entity list item to focus on graph
  document.getElementById('entity-list').addEventListener('click', (e) => {
    // Ignore if clicking on edit/delete buttons
    if (e.target.closest('.actions')) return;

    const listItem = e.target.closest('.list-item');
    if (listItem) {
      const entityName = listItem.dataset.name;
      if (entityName) {
        focusOnNode(entityName);
      }
    }
  });

  // Click on relation list item to focus on connected nodes
  document.getElementById('relation-list').addEventListener('click', (e) => {
    // Ignore if clicking on delete button
    if (e.target.closest('.actions')) return;

    const listItem = e.target.closest('.list-item');
    if (listItem) {
      const fromEntity = listItem.dataset.from;
      if (fromEntity) {
        focusOnNode(fromEntity);
      }
    }
  });

  // Modal close
  document.querySelectorAll('.modal-overlay').forEach(overlay => {
    overlay.addEventListener('click', (e) => {
      if (e.target === overlay) {
        overlay.classList.remove('active');
      }
    });
  });

  // Export button
  document.getElementById('btn-export').onclick = () => {
    editor.downloadJSONL('memory-export.jsonl');
    showToast('Data exported to memory-export.jsonl', 'success');
  };
}

// Start app when DOM ready
document.addEventListener('DOMContentLoaded', init);

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
