/**
 * Memory Graph WebSocket Client
 * Real-time updates for the Knowledge Graph UI
 *
 * @description Connects to the Memory Graph server via WebSocket
 * to receive real-time updates when entities/relations change.
 */

class MemoryGraphWS {
    /**
     * Create a new WebSocket client
     * @param {string} url - WebSocket server URL (default: ws://localhost:3030/ws)
     */
    constructor(url = 'ws://localhost:3030/ws') {
        this.url = url;
        this.ws = null;
        this.reconnectInterval = 5000;
        this.maxReconnectInterval = 30000;
        this.handlers = new Map();
        this.lastSeenSequenceId = -1;
        this.isConnected = false;
        this.reconnectTimer = null;
        this.pingTimer = null;
        this.connectionAttempts = 0;
        this.maxConnectionAttempts = 10;
    }

    /**
     * Connect to the WebSocket server
     * @returns {Promise<void>}
     */
    connect() {
        return new Promise((resolve, reject) => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                resolve();
                return;
            }

            try {
                this.ws = new WebSocket(this.url);

                this.ws.onopen = () => {
                    console.log('🔌 Connected to Memory Graph WebSocket');
                    this.isConnected = true;
                    this.reconnectInterval = 5000; // Reset on successful connect
                    this.connectionAttempts = 0;
                    this.startHeartbeat();
                    this.emit('connected');
                    resolve();
                };

                this.ws.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        this.handleMessage(data);
                    } catch (e) {
                        console.error('Failed to parse WebSocket message:', e);
                    }
                };

                this.ws.onclose = (event) => {
                    console.log('❌ WebSocket disconnected', event.code, event.reason);
                    this.isConnected = false;
                    this.stopHeartbeat();
                    this.emit('disconnected', { code: event.code, reason: event.reason });
                    this.scheduleReconnect();
                };

                this.ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                    this.emit('error', error);
                    reject(error);
                };

            } catch (error) {
                console.error('Failed to create WebSocket:', error);
                reject(error);
            }
        });
    }

    /**
     * Handle incoming WebSocket message
     * @param {Object} data - Parsed message data
     */
    handleMessage(data) {
        // Track sequence for gap detection
        if (data.sequence_id !== undefined) {
            if (this.lastSeenSequenceId >= 0 &&
                data.sequence_id > this.lastSeenSequenceId + 1) {
                const gap = data.sequence_id - this.lastSeenSequenceId - 1;
                console.warn(`⚠️ Missed ${gap} events! Requesting full refresh...`);
                this.emit('fullRefreshNeeded', {
                    lastSeen: this.lastSeenSequenceId,
                    current: data.sequence_id
                });
            }
            this.lastSeenSequenceId = data.sequence_id;
        }

        switch (data.type) {
            case 'connected':
                this.handleConnected(data);
                break;

            case 'entity_created':
                console.log('✨ Entity created:', data.payload?.name);
                this.emit('entityCreated', data.payload);
                this.showNotification(`✨ New entity: ${data.payload?.name}`, 'success');
                break;

            case 'entity_updated':
                console.log('📝 Entity updated:', data.name);
                this.emit('entityUpdated', {
                    name: data.name,
                    new_observations: data.new_observations,
                    user: data.user
                });
                this.showNotification(`📝 Updated: ${data.name}`, 'info');
                break;

            case 'entity_deleted':
                console.log('🗑️ Entity deleted:', data.name);
                this.emit('entityDeleted', data.name);
                this.showNotification(`🗑️ Deleted: ${data.name}`, 'warning');
                break;

            case 'relation_created':
                console.log('🔗 Relation created:', data.payload?.from, '→', data.payload?.to);
                this.emit('relationCreated', data.payload);
                break;

            case 'relation_deleted':
                console.log('✂️ Relation deleted:', data.from, '→', data.to);
                this.emit('relationDeleted', {
                    from: data.from,
                    to: data.to,
                    relation_type: data.relation_type
                });
                break;

            case 'batch_update':
                this.handleBatchUpdate(data.events || data.payload);
                break;

            case 'pong':
                // Heartbeat response received
                break;

            case 'error':
                console.error('Server error:', data.message);
                if (data.code === 'lagged') {
                    // We missed events, need full refresh
                    this.emit('fullRefreshNeeded', { reason: 'lagged' });
                }
                this.emit('serverError', data);
                break;

            default:
                console.log('Unknown message type:', data.type);
        }
    }

    /**
     * Handle initial connection message
     * @param {Object} data - Connection data with current_sequence_id
     */
    handleConnected(data) {
        console.log('📊 Server sequence:', data.current_sequence_id);

        if (this.lastSeenSequenceId >= 0 &&
            data.current_sequence_id > this.lastSeenSequenceId + 100) {
            // Gap too large, need full refresh
            console.log('🔄 Large gap detected, requesting full refresh...');
            this.emit('fullRefreshNeeded', {
                lastSeen: this.lastSeenSequenceId,
                current: data.current_sequence_id,
                reason: 'large_gap'
            });
        }

        this.lastSeenSequenceId = data.current_sequence_id;
        this.emit('serverConnected', data);
    }

    /**
     * Handle batch update with multiple events
     * @param {Array|Object} payload - Batch payload
     */
    handleBatchUpdate(payload) {
        if (!payload) return;

        // Handle array of events
        if (Array.isArray(payload)) {
            payload.forEach(event => this.handleMessage(event));
            this.showNotification(`📦 Batch update: ${payload.length} changes`, 'info');
            return;
        }

        // Handle structured batch payload
        let count = 0;

        if (payload.entities_created) {
            payload.entities_created.forEach(e => {
                this.emit('entityCreated', e);
                count++;
            });
        }

        if (payload.entities_updated) {
            payload.entities_updated.forEach(e => {
                this.emit('entityUpdated', e);
                count++;
            });
        }

        if (payload.entities_deleted) {
            payload.entities_deleted.forEach(name => {
                this.emit('entityDeleted', name);
                count++;
            });
        }

        if (payload.relations_created) {
            payload.relations_created.forEach(r => {
                this.emit('relationCreated', r);
                count++;
            });
        }

        if (payload.relations_deleted) {
            payload.relations_deleted.forEach(r => {
                this.emit('relationDeleted', r);
                count++;
            });
        }

        if (count > 0) {
            this.showNotification(`📦 Batch update: ${count} changes`, 'info');
        }
    }

    /**
     * Schedule reconnection with exponential backoff
     */
    scheduleReconnect() {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
        }

        this.connectionAttempts++;

        if (this.connectionAttempts > this.maxConnectionAttempts) {
            console.log('🛑 Max reconnection attempts reached');
            this.emit('maxReconnectAttempts');
            return;
        }

        console.log(`🔄 Reconnecting in ${this.reconnectInterval / 1000}s... (attempt ${this.connectionAttempts}/${this.maxConnectionAttempts})`);

        this.reconnectTimer = setTimeout(() => {
            this.connect().catch(() => {
                // Exponential backoff
                this.reconnectInterval = Math.min(
                    this.reconnectInterval * 1.5,
                    this.maxReconnectInterval
                );
            });
        }, this.reconnectInterval);
    }

    /**
     * Start heartbeat ping/pong
     */
    startHeartbeat() {
        this.stopHeartbeat();
        this.pingTimer = setInterval(() => {
            this.ping();
        }, 30000); // Every 30 seconds
    }

    /**
     * Stop heartbeat
     */
    stopHeartbeat() {
        if (this.pingTimer) {
            clearInterval(this.pingTimer);
            this.pingTimer = null;
        }
    }

    /**
     * Send ping to server
     */
    ping() {
        this.send({ type: 'ping' });
    }

    /**
     * Subscribe to a channel
     * @param {string} channel - Channel name (entities, relations, all)
     * @param {Object} filter - Optional filter
     */
    subscribe(channel = 'all', filter = {}) {
        this.send({
            type: 'subscribe',
            channel,
            filter
        });
    }

    /**
     * Unsubscribe from a channel
     * @param {string} channel - Channel name
     */
    unsubscribe(channel) {
        this.send({
            type: 'unsubscribe',
            channel
        });
    }

    /**
     * Send message to server
     * @param {Object} data - Data to send
     */
    send(data) {
        if (this.ws?.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(data));
        }
    }

    /**
     * Register event handler
     * @param {string} eventType - Event type
     * @param {Function} handler - Handler function
     * @returns {MemoryGraphWS} - For chaining
     */
    on(eventType, handler) {
        if (!this.handlers.has(eventType)) {
            this.handlers.set(eventType, []);
        }
        this.handlers.get(eventType).push(handler);
        return this; // Chainable
    }

    /**
     * Remove event handler
     * @param {string} eventType - Event type
     * @param {Function} handler - Handler to remove
     */
    off(eventType, handler) {
        if (this.handlers.has(eventType)) {
            const handlers = this.handlers.get(eventType);
            const index = handlers.indexOf(handler);
            if (index > -1) {
                handlers.splice(index, 1);
            }
        }
    }

    /**
     * Emit event to handlers
     * @param {string} eventType - Event type
     * @param {*} data - Event data
     */
    emit(eventType, data) {
        const handlers = this.handlers.get(eventType) || [];
        handlers.forEach(handler => {
            try {
                handler(data);
            } catch (e) {
                console.error(`Error in ${eventType} handler:`, e);
            }
        });
    }

    /**
     * Show notification (override in app)
     * @param {string} message - Message to show
     * @param {string} type - Notification type (success, info, warning, error)
     */
    showNotification(message, type = 'info') {
        // Use app's showToast if available, otherwise console.log
        if (typeof showToast === 'function') {
            showToast(message, type);
        } else {
            console.log(`[${type}] ${message}`);
        }
    }

    /**
     * Disconnect from server
     */
    disconnect() {
        this.stopHeartbeat();

        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }

        if (this.ws) {
            this.ws.close(1000, 'Client disconnect');
            this.ws = null;
        }

        this.isConnected = false;
    }

    /**
     * Get connection status
     * @returns {boolean}
     */
    get connected() {
        return this.isConnected && this.ws?.readyState === WebSocket.OPEN;
    }
}

// Global WebSocket instance
let memoryWS = null;

/**
 * Initialize WebSocket connection for the UI
 * @param {string} wsUrl - WebSocket URL (optional, defaults to ws://localhost:3030/ws)
 */
function initWebSocket(wsUrl) {
    // Determine WebSocket URL
    const url = wsUrl || getWebSocketUrl();

    memoryWS = new MemoryGraphWS(url);

    // Setup event handlers
    memoryWS
        .on('connected', () => {
            updateConnectionStatus(true);
        })
        .on('disconnected', () => {
            updateConnectionStatus(false);
        })
        .on('entityCreated', (entity) => {
            if (typeof editor !== 'undefined' && editor) {
                // Add to local data
                editor.entities.push(entity);
                // Rebuild graph
                rebuildGraphDelayed();
            }
        })
        .on('entityUpdated', (data) => {
            if (typeof editor !== 'undefined' && editor) {
                // Update local data
                const entity = editor.entities.find(e => e.name === data.name);
                if (entity && data.new_observations) {
                    entity.observations = [...entity.observations, ...data.new_observations];
                }
                // Update details panel if this entity is selected
                if (selectedNode === `entity:${data.name}`) {
                    const attrs = graph?.getNodeAttributes(selectedNode);
                    if (attrs && typeof showNodeDetail === 'function') {
                        showNodeDetail(data.name, attrs);
                    }
                }
            }
        })
        .on('entityDeleted', (name) => {
            if (typeof editor !== 'undefined' && editor) {
                // Remove from local data
                editor.entities = editor.entities.filter(e => e.name !== name);
                editor.relations = editor.relations.filter(
                    r => r.from !== name && r.to !== name
                );
                // Clear selection if deleted node was selected
                if (selectedNode === `entity:${name}`) {
                    if (typeof closeNodeDetail === 'function') {
                        closeNodeDetail();
                    }
                }
                // Rebuild graph
                rebuildGraphDelayed();
            }
        })
        .on('relationCreated', (relation) => {
            if (typeof editor !== 'undefined' && editor) {
                // Add to local data
                editor.relations.push(relation);
                // Rebuild graph
                rebuildGraphDelayed();
            }
        })
        .on('relationDeleted', (data) => {
            if (typeof editor !== 'undefined' && editor) {
                // Remove from local data
                editor.relations = editor.relations.filter(
                    r => !(r.from === data.from && r.to === data.to && r.relationType === data.relation_type)
                );
                // Rebuild graph
                rebuildGraphDelayed();
            }
        })
        .on('fullRefreshNeeded', async (info) => {
            console.log('🔄 Full refresh needed:', info);
            showToast('Refreshing data...', 'info');

            // Reload data from server
            if (typeof editor !== 'undefined' && editor) {
                try {
                    await editor.loadFromURL('../memory.jsonl');
                    initFilters();
                    rebuildGraph();
                    updateStats();
                    showToast('Data refreshed', 'success');
                } catch (error) {
                    console.error('Failed to refresh data:', error);
                    showToast('Failed to refresh data', 'error');
                }
            }
        })
        .on('serverError', (error) => {
            console.error('Server error:', error);
            showToast(`Server error: ${error.message}`, 'error');
        })
        .on('maxReconnectAttempts', () => {
            showToast('Unable to connect to server. Please refresh the page.', 'error');
        });

    // Try to connect
    memoryWS.connect().catch(error => {
        console.log('WebSocket connection failed (server may not be running in HTTP mode):', error);
    });

    return memoryWS;
}

/**
 * Get WebSocket URL based on current location
 * @returns {string} WebSocket URL
 */
function getWebSocketUrl() {
    // Default to localhost:3030 for development
    // In production, would use window.location to determine host
    const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsHost = window.location.hostname || 'localhost';
    const wsPort = 3030; // Memory Graph WebSocket port

    return `${wsProtocol}//${wsHost}:${wsPort}/ws`;
}

/**
 * Update connection status indicator in UI
 * @param {boolean} connected - Whether connected
 */
function updateConnectionStatus(connected) {
    const indicator = document.getElementById('ws-status');
    if (indicator) {
        indicator.className = connected ? 'ws-status connected' : 'ws-status disconnected';
        indicator.title = connected ? 'Real-time updates active' : 'Disconnected - updates paused';
    }
}

// Debounced graph rebuild to avoid too many rebuilds
let rebuildTimeout = null;
function rebuildGraphDelayed() {
    if (rebuildTimeout) {
        clearTimeout(rebuildTimeout);
    }
    rebuildTimeout = setTimeout(() => {
        if (typeof rebuildGraph === 'function') {
            rebuildGraph();
            updateStats();
        }
    }, 100); // Debounce 100ms
}

// Export for use in other modules
if (typeof window !== 'undefined') {
    window.MemoryGraphWS = MemoryGraphWS;
    window.initWebSocket = initWebSocket;
    window.memoryWS = memoryWS;
}
