// Editor module for entities and relations

class DataEditor {
  constructor() {
    this.entities = [];
    this.relations = [];
    this.selectedEntity = null;
    this.selectedRelation = null;
    this.onDataChange = null;
  }

  // Load data from JSONL
  async loadFromURL(url) {
    const res = await fetch(url);
    const text = await res.text();
    const data = text.split('\n').filter(l => l.trim()).map(l => JSON.parse(l));

    this.entities = data.filter(item => item.entityType);
    this.relations = data.filter(item => item.relationType);

    return { entities: this.entities, relations: this.relations };
  }

  // Entity CRUD
  addEntity(entity) {
    if (!entity.name || !entity.entityType) {
      throw new Error('Entity must have name and entityType');
    }
    if (this.entities.find(e => e.name === entity.name)) {
      throw new Error('Entity with this name already exists');
    }
    this.entities.push({
      name: entity.name,
      entityType: entity.entityType,
      observations: entity.observations || [],
      createdAt: Math.floor(Date.now() / 1000)
    });
    this._notifyChange();
    return true;
  }

  updateEntity(name, updates) {
    const entity = this.entities.find(e => e.name === name);
    if (!entity) {
      throw new Error('Entity not found');
    }

    if (updates.name && updates.name !== name) {
      // Update relations that reference this entity
      this.relations.forEach(rel => {
        if (rel.from === name) rel.from = updates.name;
        if (rel.to === name) rel.to = updates.name;
      });
    }

    Object.assign(entity, updates, { updatedAt: Math.floor(Date.now() / 1000) });
    this._notifyChange();
    return true;
  }

  deleteEntity(name) {
    const idx = this.entities.findIndex(e => e.name === name);
    if (idx === -1) {
      throw new Error('Entity not found');
    }

    // Remove related relations
    this.relations = this.relations.filter(r => r.from !== name && r.to !== name);
    this.entities.splice(idx, 1);
    this._notifyChange();
    return true;
  }

  // Relation CRUD
  addRelation(relation) {
    if (!relation.from || !relation.to || !relation.relationType) {
      throw new Error('Relation must have from, to, and relationType');
    }
    if (!this.entities.find(e => e.name === relation.from)) {
      throw new Error(`Entity "${relation.from}" not found`);
    }
    if (!this.entities.find(e => e.name === relation.to)) {
      throw new Error(`Entity "${relation.to}" not found`);
    }
    if (this.relations.find(r => r.from === relation.from && r.to === relation.to && r.relationType === relation.relationType)) {
      throw new Error('This relation already exists');
    }

    this.relations.push({
      from: relation.from,
      to: relation.to,
      relationType: relation.relationType,
      createdAt: Math.floor(Date.now() / 1000)
    });
    this._notifyChange();
    return true;
  }

  updateRelation(from, to, relationType, updates) {
    const rel = this.relations.find(r =>
      r.from === from && r.to === to && r.relationType === relationType
    );
    if (!rel) {
      throw new Error('Relation not found');
    }

    Object.assign(rel, updates);
    this._notifyChange();
    return true;
  }

  deleteRelation(from, to, relationType) {
    const idx = this.relations.findIndex(r =>
      r.from === from && r.to === to && r.relationType === relationType
    );
    if (idx === -1) {
      throw new Error('Relation not found');
    }

    this.relations.splice(idx, 1);
    this._notifyChange();
    return true;
  }

  // Observations
  addObservation(entityName, observation) {
    const entity = this.entities.find(e => e.name === entityName);
    if (!entity) {
      throw new Error('Entity not found');
    }
    if (!entity.observations) {
      entity.observations = [];
    }
    entity.observations.push(observation);
    entity.updatedAt = Math.floor(Date.now() / 1000);
    this._notifyChange();
    return true;
  }

  removeObservation(entityName, observation) {
    const entity = this.entities.find(e => e.name === entityName);
    if (!entity || !entity.observations) {
      throw new Error('Entity not found');
    }
    const idx = entity.observations.indexOf(observation);
    if (idx > -1) {
      entity.observations.splice(idx, 1);
      entity.updatedAt = Math.floor(Date.now() / 1000);
      this._notifyChange();
    }
    return true;
  }

  // Export to JSONL
  exportToJSONL() {
    const lines = [];
    this.entities.forEach(e => lines.push(JSON.stringify(e)));
    this.relations.forEach(r => lines.push(JSON.stringify(r)));
    return lines.join('\n');
  }

  // Download JSONL
  downloadJSONL(filename = 'memory.jsonl') {
    const content = this.exportToJSONL();
    const blob = new Blob([content], { type: 'application/jsonl' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  // Get entity types
  getEntityTypes() {
    return [...new Set(this.entities.map(e => e.entityType))].sort();
  }

  // Get relation types
  getRelationTypes() {
    return [...new Set(this.relations.map(r => r.relationType))].sort();
  }

  // Search entities
  searchEntities(query) {
    const q = query.toLowerCase();
    return this.entities.filter(e =>
      e.name.toLowerCase().includes(q) ||
      e.entityType.toLowerCase().includes(q) ||
      (e.observations || []).some(o => o.toLowerCase().includes(q))
    );
  }

  // Get relations for entity
  getRelationsForEntity(entityName) {
    return {
      outgoing: this.relations.filter(r => r.from === entityName),
      incoming: this.relations.filter(r => r.to === entityName)
    };
  }

  _notifyChange() {
    if (this.onDataChange) {
      this.onDataChange(this.entities, this.relations);
    }
  }
}

// Toast notifications
function showToast(message, type = 'info') {
  let container = document.querySelector('.toast-container');
  if (!container) {
    container = document.createElement('div');
    container.className = 'toast-container';
    document.body.appendChild(container);
  }

  const toast = document.createElement('div');
  toast.className = `toast ${type}`;
  toast.innerHTML = `
    <span>${type === 'success' ? '✓' : type === 'error' ? '✕' : 'ℹ'}</span>
    <span>${message}</span>
  `;
  container.appendChild(toast);

  setTimeout(() => {
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// Export
window.DataEditor = DataEditor;
window.showToast = showToast;
