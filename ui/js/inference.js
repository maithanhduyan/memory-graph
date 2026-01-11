// Inference engine module

const relationConfidence = {
  'depends_on': 0.95,
  'implements': 0.90,
  'affects': 0.85,
  'part_of': 0.90,
  'owns': 0.80,
  'threatens': 0.75,
  'related_to': 0.60,
  'will_implement': 0.70,
  'proposes': 0.65,
  'caused_by': 0.85,
  'fixes': 0.90,
  'includes': 0.85,
  'satisfied_by': 0.80,
  'supports': 0.75,
  'develops': 0.85,
  'made': 0.80,
  'planned_for': 0.70
};

// Run BFS-based inference
function runInference(entityName, relations, maxDepth, minConfidence) {
  const results = [];
  const visited = new Set();
  const queue = [{
    node: entityName,
    path: [entityName],
    pathRelations: [],
    depth: 0,
    confidence: 1.0
  }];

  visited.add(entityName);

  while (queue.length > 0) {
    const { node, path, pathRelations, depth, confidence } = queue.shift();

    if (depth >= maxDepth) continue;

    // Find outgoing relations
    relations.forEach(rel => {
      if (rel.from === node && !visited.has(rel.to)) {
        const relConf = relationConfidence[rel.relationType] || 0.5;
        const newConf = confidence * relConf;

        if (depth > 0 && newConf >= minConfidence) {
          // Create inferred relation type
          const inferredType = deriveInferredType(pathRelations.concat(rel.relationType));

          results.push({
            from: entityName,
            to: rel.to,
            relationType: inferredType,
            confidence: newConf,
            path: [...path, rel.to],
            pathRelations: [...pathRelations, rel.relationType],
            depth: depth + 1
          });
        }

        visited.add(rel.to);
        queue.push({
          node: rel.to,
          path: [...path, rel.to],
          pathRelations: [...pathRelations, rel.relationType],
          depth: depth + 1,
          confidence: newConf
        });
      }
    });

    // Also check incoming relations for bidirectional inference
    relations.forEach(rel => {
      if (rel.to === node && !visited.has(rel.from)) {
        const relConf = (relationConfidence[rel.relationType] || 0.5) * 0.8; // Lower confidence for reverse
        const newConf = confidence * relConf;

        if (depth > 0 && newConf >= minConfidence) {
          const inferredType = deriveInferredType(pathRelations.concat(`reverse_${rel.relationType}`));

          results.push({
            from: entityName,
            to: rel.from,
            relationType: inferredType,
            confidence: newConf,
            path: [...path, rel.from],
            pathRelations: [...pathRelations, `←${rel.relationType}`],
            depth: depth + 1,
            isReverse: true
          });
        }

        visited.add(rel.from);
        queue.push({
          node: rel.from,
          path: [...path, rel.from],
          pathRelations: [...pathRelations, `←${rel.relationType}`],
          depth: depth + 1,
          confidence: newConf
        });
      }
    });
  }

  // Sort by confidence and limit
  return results.sort((a, b) => b.confidence - a.confidence).slice(0, 30);
}

// Derive inferred relation type from path
function deriveInferredType(pathRelations) {
  if (pathRelations.length === 1) {
    return `transitively_${pathRelations[0]}`;
  }

  // Special inference rules
  const pathStr = pathRelations.join('→');

  // Rule: A depends_on B, B depends_on C => A transitively_depends_on C
  if (pathRelations.every(r => r === 'depends_on')) {
    return 'transitively_depends_on';
  }

  // Rule: A implements B, B part_of C => A contributes_to C
  if (pathRelations.includes('implements') && pathRelations.includes('part_of')) {
    return 'contributes_to';
  }

  // Rule: A owns B, B depends_on C => A indirectly_owns C
  if (pathRelations[0] === 'owns') {
    return 'indirectly_responsible_for';
  }

  // Rule: Risk threatens Milestone, Milestone includes Feature => Risk threatens Feature
  if (pathRelations.includes('threatens') && pathRelations.includes('includes')) {
    return 'indirectly_threatens';
  }

  // Rule: A affects B, B affects C => A indirectly_affects C
  if (pathRelations.every(r => r === 'affects' || r === 'threatens')) {
    return 'indirectly_affects';
  }

  // Default: combine relation types
  return `inferred_via_${pathRelations.length}_hops`;
}

// Format path for display
function formatPath(path, pathRelations) {
  let result = [];
  for (let i = 0; i < path.length; i++) {
    result.push(`<span class="entity">${path[i]}</span>`);
    if (i < pathRelations.length) {
      result.push(`<span class="arrow">—[${pathRelations[i]}]→</span>`);
    }
  }
  return result.join('');
}

// Get confidence color
function getConfidenceColor(confidence) {
  if (confidence > 0.8) return '#22c55e';
  if (confidence > 0.6) return '#eab308';
  return '#f97316';
}

// Export functions
window.InferenceModule = {
  runInference,
  formatPath,
  getConfidenceColor,
  relationConfidence
};
