(async ()=>{
    const S = {};
    async function k() {
        let e;
        return (!S || S) && (e = await import("./Cargo-M3382VZc-Dfw4tXwh-9dgrhKKr.js"), e && typeof e.__wbg_init == "function" && await e.__wbg_init({
            module_or_path: e.__wasm_url
        })), e;
    }
    const C = Object.freeze({
        INIT: "init",
        INIT_MOCK: "initMock",
        CALL_METHOD: "callMethod",
        EXECUTE_CALLBACK: "executeCallback"
    }), N = Object.freeze({
        GET_KEY: "getKey",
        INSERT_KEY: "insertKey",
        SIGN: "sign"
    }), u = Object.freeze({
        CREATE_CLIENT: "createClient",
        APPLY_TRANSACTION: "applyTransaction",
        EXECUTE_TRANSACTION: "executeTransaction",
        PROVE_TRANSACTION: "proveTransaction",
        SUBMIT_NEW_TRANSACTION: "submitNewTransaction",
        SUBMIT_NEW_TRANSACTION_MOCK: "submitNewTransactionMock",
        SUBMIT_NEW_TRANSACTION_WITH_PROVER: "submitNewTransactionWithProver",
        SUBMIT_NEW_TRANSACTION_WITH_PROVER_MOCK: "submitNewTransactionWithProverMock",
        SYNC_STATE: "syncState",
        SYNC_STATE_MOCK: "syncStateMock"
    });
    let I = null;
    const w = async ()=>{
        if (I || (I = await k()), !I) throw new Error("Miden WASM bindings are unavailable in the worker environment.");
        return I;
    }, g = (e)=>{
        if (typeof e == "string") return e;
        try {
            return JSON.stringify(e);
        } catch  {
            return String(e);
        }
    }, h = (e)=>e instanceof Error ? {
            name: e.name,
            message: e.message,
            stack: e.stack,
            cause: e.cause ? h(e.cause) : void 0,
            code: e.code
        } : typeof e == "object" && e !== null ? {
            name: e.name ?? "Error",
            message: e.message ?? g(e)
        } : {
            name: "Error",
            message: g(e)
        };
    let i = null, f = null, A = !1, b = [], R = !1, l = new Map;
    const M = 3e4, z = {
        getKey: async (e)=>new Promise((s, n)=>{
                const a = `${N.GET_KEY}-${Date.now()}-${Math.random()}`, t = setTimeout(()=>{
                    l.has(a) && (l.delete(a), n(new Error(`Callback ${a} timed out`)));
                }, M);
                l.set(a, {
                    resolve: s,
                    reject: n,
                    timeoutId: t
                }), self.postMessage({
                    action: C.EXECUTE_CALLBACK,
                    callbackType: N.GET_KEY,
                    args: [
                        e
                    ],
                    requestId: a
                });
            }),
        insertKey: async (e, s)=>new Promise((n, a)=>{
                const t = `${N.INSERT_KEY}-${Date.now()}-${Math.random()}`, r = setTimeout(()=>{
                    l.has(t) && (l.delete(t), a(new Error(`Callback ${t} timed out`)));
                }, M);
                l.set(t, {
                    resolve: n,
                    reject: a,
                    timeoutId: r
                }), self.postMessage({
                    action: C.EXECUTE_CALLBACK,
                    callbackType: N.INSERT_KEY,
                    args: [
                        e,
                        s
                    ],
                    requestId: t
                });
            }),
        sign: async (e, s)=>new Promise((n, a)=>{
                const t = `${N.SIGN}-${Date.now()}-${Math.random()}`, r = setTimeout(()=>{
                    l.has(t) && (l.delete(t), a(new Error(`Callback ${t} timed out`)));
                }, M);
                l.set(t, {
                    resolve: n,
                    reject: a,
                    timeoutId: r
                }), self.postMessage({
                    action: C.EXECUTE_CALLBACK,
                    callbackType: N.SIGN,
                    args: [
                        e,
                        s
                    ],
                    requestId: t
                });
            })
    }, p = {
        [u.SYNC_STATE]: async ()=>(await i.syncStateImpl()).serialize().buffer,
        [u.APPLY_TRANSACTION]: async (e)=>{
            const s = await w(), [n, a] = e, t = new Uint8Array(n), r = s.TransactionResult.deserialize(t);
            return (await i.applyTransaction(r, a)).serialize().buffer;
        },
        [u.EXECUTE_TRANSACTION]: async (e)=>{
            const s = await w(), [n, a] = e, t = s.AccountId.fromHex(n), r = new Uint8Array(a), o = s.TransactionRequest.deserialize(r);
            return (await i.executeTransaction(t, o)).serialize().buffer;
        },
        [u.PROVE_TRANSACTION]: async (e)=>{
            const s = await w(), [n, a] = e, t = new Uint8Array(n), r = s.TransactionResult.deserialize(t), o = a ? s.TransactionProver.deserialize(a) : null;
            return (o ? await i.proveTransactionWithProver(r, o) : await i.proveTransaction(r)).serialize().buffer;
        },
        [u.SUBMIT_NEW_TRANSACTION]: async (e)=>{
            const s = await w(), [n, a] = e, t = s.AccountId.fromHex(n), r = new Uint8Array(a), o = s.TransactionRequest.deserialize(r), c = await i.executeTransaction(t, o), T = c.id().toHex(), d = await i.proveTransaction(c), m = await i.submitProvenTransaction(d, c), _ = await i.applyTransaction(c, m);
            return {
                transactionId: T,
                submissionHeight: m,
                serializedTransactionResult: c.serialize().buffer,
                serializedTransactionUpdate: _.serialize().buffer
            };
        },
        [u.SUBMIT_NEW_TRANSACTION_WITH_PROVER]: async (e)=>{
            const s = await w(), [n, a, t] = e, r = s.AccountId.fromHex(n), o = new Uint8Array(a), c = s.TransactionRequest.deserialize(o), T = t ? s.TransactionProver.deserialize(t) : null, d = await i.executeTransaction(r, c), m = d.id().toHex(), _ = T ? await i.proveTransactionWithProver(d, T) : await i.proveTransaction(d), E = await i.submitProvenTransaction(_, d), y = await i.applyTransaction(d, E);
            return {
                transactionId: m,
                submissionHeight: E,
                serializedTransactionResult: d.serialize().buffer,
                serializedTransactionUpdate: y.serialize().buffer
            };
        }
    };
    p[u.SYNC_STATE_MOCK] = async (e)=>{
        let [s, n] = e;
        return s = new Uint8Array(s), n = n ? new Uint8Array(n) : null, await i.createMockClient(f, s, n), await p[u.SYNC_STATE]();
    };
    p[u.SUBMIT_NEW_TRANSACTION_MOCK] = async (e)=>{
        const s = await w();
        let n = e.pop(), a = e.pop();
        a = new Uint8Array(a), n = n ? new Uint8Array(n) : null, i = new s.WebClient, await i.createMockClient(f, a, n);
        const t = await p[u.SUBMIT_NEW_TRANSACTION](e);
        return {
            transactionId: t.transactionId,
            submissionHeight: t.submissionHeight,
            serializedTransactionResult: t.serializedTransactionResult,
            serializedTransactionUpdate: t.serializedTransactionUpdate,
            serializedMockChain: i.serializeMockChain().buffer,
            serializedMockNoteTransportNode: i.serializeMockNoteTransportNode().buffer
        };
    };
    p[u.SUBMIT_NEW_TRANSACTION_WITH_PROVER_MOCK] = async (e)=>{
        const s = await w();
        let n = e.pop(), a = e.pop();
        a = new Uint8Array(a), n = n ? new Uint8Array(n) : null, i = new s.WebClient, await i.createMockClient(f, a, n);
        const t = await p[u.SUBMIT_NEW_TRANSACTION_WITH_PROVER](e);
        return {
            transactionId: t.transactionId,
            submissionHeight: t.submissionHeight,
            serializedTransactionResult: t.serializedTransactionResult,
            serializedTransactionUpdate: t.serializedTransactionUpdate,
            serializedMockChain: i.serializeMockChain().buffer,
            serializedMockNoteTransportNode: i.serializeMockNoteTransportNode().buffer
        };
    };
    async function O(e) {
        const { action: s, args: n, methodName: a, requestId: t } = e.data;
        try {
            if (s === C.INIT) {
                const [r, o, c, T, d, m, _, E] = n, y = await w();
                E && y.setupLogging(E), i = new y.WebClient, d || m || _ ? await i.createClientWithExternalKeystore(r, o, c, T, d ? z.getKey : void 0, m ? z.insertKey : void 0, _ ? z.sign : void 0) : await i.createClient(r, o, c, T), f = c, A = !0, self.postMessage({
                    ready: !0
                });
                return;
            } else if (s === C.INIT_MOCK) {
                const [r, o] = n, c = await w();
                o && c.setupLogging(o), i = new c.WebClient, await i.createMockClient(r, void 0, void 0), f = r, A = !0, self.postMessage({
                    ready: !0
                });
                return;
            } else if (s === C.CALL_METHOD) {
                if (!A) throw new Error("Worker is not ready. Please initialize first.");
                if (!i) throw new Error("WebClient not initialized in worker.");
                const r = p[a];
                if (!r) throw new Error(`Unsupported method: ${a}`);
                const o = await r(n);
                self.postMessage({
                    requestId: t,
                    result: o,
                    methodName: a
                });
                return;
            } else throw new Error(`Unsupported action: ${s}`);
        } catch (r) {
            const o = h(r);
            console.error("WORKER: Error occurred - %s", o.message, r), self.postMessage({
                requestId: t,
                error: o,
                methodName: a
            });
        }
    }
    async function U() {
        if (R || b.length === 0) return;
        R = !0;
        const e = b.shift();
        try {
            await O(e);
        } finally{
            R = !1, U();
        }
    }
    self.onmessage = (e)=>{
        if (e.data.callbackRequestId && l.has(e.data.callbackRequestId)) {
            const { callbackRequestId: s, callbackResult: n, callbackError: a } = e.data, { resolve: t, reject: r, timeoutId: o } = l.get(s);
            clearTimeout(o), l.delete(s), a ? r(new Error(a)) : t(n);
            return;
        }
        b.push(e), U();
    };
    self.postMessage({
        loaded: !0
    });
})();
