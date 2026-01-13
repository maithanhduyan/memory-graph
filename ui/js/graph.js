// Graph visualization module

const typeColors = {
  "Project": "#facc15",
  "Person": "#4ade80",
  "Developer": "#4ade80",
  "Task": "#f87171",
  "Decision": "#a78bfa",
  "Technology": "#22d3ee",
  "Component": "#fb923c",
  "Feature": "#34d399",
  "Milestone": "#f472b6",
  "Requirement": "#60a5fa",
  "Risk": "#ef4444",
  "Bug": "#dc2626",
  "Research": "#fbbf24",
  "Insight": "#c084fc",
  "Convention": "#2dd4bf",
  "Proposal": "#fb7185",
  "Module": "#818cf8",
  "EntityType": "#64748b",
  "Lesson": "#fcd34d"
};

function colorByType(type) {
  return typeColors[type] || "#94a3b8";
}

// Build graph from data
function buildGraph(entities, relations, showObservations = false) {
  const graph = new graphology.Graph();
  const entityTypes = new Set();

  entities.forEach(e => entityTypes.add(e.entityType));

  // Type hub nodes
  let typeIndex = 0;
  entityTypes.forEach(type => {
    const angle = (2 * Math.PI * typeIndex) / entityTypes.size;
    graph.addNode(`type:${type}`, {
      label: type,
      x: Math.cos(angle) * 5,
      y: Math.sin(angle) * 5,
      size: 35,
      color: colorByType(type),
      nodeType: 'entityType'
    });
    typeIndex++;
  });

  // Group entities by type
  const entitiesPerType = {};
  entities.forEach(e => {
    if (!entitiesPerType[e.entityType]) entitiesPerType[e.entityType] = [];
    entitiesPerType[e.entityType].push(e);
  });

  // Entity nodes
  Object.entries(entitiesPerType).forEach(([type, typeEntities]) => {
    typeEntities.forEach((e, idx) => {
      const nodeId = `entity:${e.name}`;
      const typeNode = `type:${type}`;
      const typeAttrs = graph.getNodeAttributes(typeNode);

      const count = typeEntities.length;
      const angle = (2 * Math.PI * idx) / count;
      const radius = 1.2 + (count > 10 ? count * 0.08 : count * 0.12);

      graph.addNode(nodeId, {
        label: e.name.length > 25 ? e.name.substring(0, 25) + '...' : e.name,
        fullName: e.name,
        x: typeAttrs.x + Math.cos(angle) * radius,
        y: typeAttrs.y + Math.sin(angle) * radius,
        size: 14,
        color: colorByType(type),
        nodeType: 'entity',
        entityType: type,
        observations: e.observations || []
      });

      graph.addEdge(nodeId, typeNode, { size: 1, color: "#334155", edgeType: 'type' });

      // Observation nodes
      if (showObservations && e.observations) {
        e.observations.slice(0, 3).forEach((obs, obsIdx) => {
          const obsId = `obs:${e.name}:${obsIdx}`;
          const obsAngle = (2 * Math.PI * obsIdx) / Math.min(e.observations.length, 3);
          const obsOffset = 0.3;

          const entityAttrs = graph.getNodeAttributes(nodeId);
          graph.addNode(obsId, {
            label: obs.length > 15 ? obs.substring(0, 15) + '...' : obs,
            x: entityAttrs.x + Math.cos(obsAngle) * obsOffset,
            y: entityAttrs.y + Math.sin(obsAngle) * obsOffset,
            size: 4,
            color: "#475569",
            nodeType: 'observation',
            fullText: obs
          });

          graph.addEdge(nodeId, obsId, { size: 0.3, color: "#1e293b", edgeType: 'observation' });
        });
      }
    });
  });

  // Relation edges
  relations.forEach(rel => {
    const fromNode = `entity:${rel.from}`;
    const toNode = `entity:${rel.to}`;

    if (graph.hasNode(fromNode) && graph.hasNode(toNode)) {
      const edgeKey = `rel:${rel.from}->${rel.to}:${rel.relationType}`;
      if (!graph.hasEdge(edgeKey)) {
        graph.addEdgeWithKey(edgeKey, fromNode, toNode, {
          size: 2,
          color: "#f59e0b",
          label: rel.relationType,
          edgeType: 'relation',
          relationType: rel.relationType
        });
      }
    }
  });

  return graph;
}

// Calculate stats
function calculateStats(entities, relations) {
  const types = new Set(entities.map(e => e.entityType));
  const observations = entities.reduce((sum, e) => sum + (e.observations?.length || 0), 0);

  return {
    entities: entities.length,
    relations: relations.length,
    types: types.size,
    observations: observations
  };
}

// Export functions
window.GraphModule = {
  colorByType,
  buildGraph,
  calculateStats,
  typeColors
};
