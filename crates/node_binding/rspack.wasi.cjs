/* eslint-disable */
/* prettier-ignore */

/* auto-generated by NAPI-RS */

const __nodeFs = require('node:fs')
const __nodePath = require('node:path')
const { WASI: __nodeWASI } = require('node:wasi')
const { Worker } = require('node:worker_threads')

const {
  instantiateNapiModuleSync: __emnapiInstantiateNapiModuleSync,
  getDefaultContext: __emnapiGetDefaultContext,
  createOnMessage: __wasmCreateOnMessageForFsProxy,
} = require('@napi-rs/wasm-runtime')

const __rootDir = __nodePath.parse(process.cwd()).root

const __wasi = new __nodeWASI({
  version: 'preview1',
  env: process.env,
  preopens: {
    [__rootDir]: __rootDir,
  }
})

const __emnapiContext = __emnapiGetDefaultContext()

const __sharedMemory = new WebAssembly.Memory({
  initial: 16384,
  maximum: 65536,
  shared: true,
})

let __wasmFilePath = __nodePath.join(__dirname, 'rspack.wasm32-wasi.wasm')
const __wasmDebugFilePath = __nodePath.join(__dirname, 'rspack.wasm32-wasi.debug.wasm')

if (__nodeFs.existsSync(__wasmDebugFilePath)) {
  __wasmFilePath = __wasmDebugFilePath
} else if (!__nodeFs.existsSync(__wasmFilePath)) {
  try {
    __wasmFilePath = __nodePath.resolve('@rspack/binding-wasm32-wasi')
  } catch {
    throw new Error('Cannot find rspack.wasm32-wasi.wasm file, and @rspack/binding-wasm32-wasi package is not installed.')
  }
}

const { instance: __napiInstance, module: __wasiModule, napiModule: __napiModule } = __emnapiInstantiateNapiModuleSync(__nodeFs.readFileSync(__wasmFilePath), {
  context: __emnapiContext,
  asyncWorkPoolSize: (function() {
    const threadsSizeFromEnv = Number(process.env.NAPI_RS_ASYNC_WORK_POOL_SIZE ?? process.env.UV_THREADPOOL_SIZE)
    // NaN > 0 is false
    if (threadsSizeFromEnv > 0) {
      return threadsSizeFromEnv
    } else {
      return 4
    }
  })(),
  reuseWorker: true,
  wasi: __wasi,
  onCreateWorker() {
    const worker = new Worker(__nodePath.join(__dirname, 'wasi-worker.mjs'), {
      env: process.env,
    })
    worker.onmessage = ({ data }) => {
      __wasmCreateOnMessageForFsProxy(__nodeFs)(data)
    }
    return worker
  },
  overwriteImports(importObject) {
    importObject.env = {
      ...importObject.env,
      ...importObject.napi,
      ...importObject.emnapi,
      memory: __sharedMemory,
    }
    return importObject
  },
  beforeInit({ instance }) {
    for (const name of Object.keys(instance.exports)) {
      if (name.startsWith('__napi_register__')) {
        instance.exports[name]()
      }
    }
  },
})

module.exports.AsyncDependenciesBlock = __napiModule.exports.AsyncDependenciesBlock
module.exports.Chunks = __napiModule.exports.Chunks
module.exports.ConcatenatedModule = __napiModule.exports.ConcatenatedModule
module.exports.ContextModule = __napiModule.exports.ContextModule
module.exports.Dependency = __napiModule.exports.Dependency
module.exports.EntryDataDto = __napiModule.exports.EntryDataDto
module.exports.EntryDataDTO = __napiModule.exports.EntryDataDTO
module.exports.EntryDependency = __napiModule.exports.EntryDependency
module.exports.EntryOptionsDto = __napiModule.exports.EntryOptionsDto
module.exports.EntryOptionsDTO = __napiModule.exports.EntryOptionsDTO
module.exports.ExternalModule = __napiModule.exports.ExternalModule
module.exports.JsChunk = __napiModule.exports.JsChunk
module.exports.JsChunkGraph = __napiModule.exports.JsChunkGraph
module.exports.JsChunkGroup = __napiModule.exports.JsChunkGroup
module.exports.JsCompilation = __napiModule.exports.JsCompilation
module.exports.JsCompiler = __napiModule.exports.JsCompiler
module.exports.JsContextModuleFactoryAfterResolveData = __napiModule.exports.JsContextModuleFactoryAfterResolveData
module.exports.JsContextModuleFactoryBeforeResolveData = __napiModule.exports.JsContextModuleFactoryBeforeResolveData
module.exports.JsDependencies = __napiModule.exports.JsDependencies
module.exports.JsEntries = __napiModule.exports.JsEntries
module.exports.JsExportsInfo = __napiModule.exports.JsExportsInfo
module.exports.JsModuleGraph = __napiModule.exports.JsModuleGraph
module.exports.JsResolver = __napiModule.exports.JsResolver
module.exports.JsResolverFactory = __napiModule.exports.JsResolverFactory
module.exports.JsStats = __napiModule.exports.JsStats
module.exports.Module = __napiModule.exports.Module
module.exports.ModuleGraphConnection = __napiModule.exports.ModuleGraphConnection
module.exports.NormalModule = __napiModule.exports.NormalModule
module.exports.RawExternalItemFnCtx = __napiModule.exports.RawExternalItemFnCtx
module.exports.BuiltinPluginName = __napiModule.exports.BuiltinPluginName
module.exports.cleanupGlobalTrace = __napiModule.exports.cleanupGlobalTrace
module.exports.formatDiagnostic = __napiModule.exports.formatDiagnostic
module.exports.JsLoaderState = __napiModule.exports.JsLoaderState
module.exports.JsRspackSeverity = __napiModule.exports.JsRspackSeverity
module.exports.RawRuleSetConditionType = __napiModule.exports.RawRuleSetConditionType
module.exports.registerGlobalTrace = __napiModule.exports.registerGlobalTrace
module.exports.RegisterJsTapKind = __napiModule.exports.RegisterJsTapKind
module.exports.shutdownAsyncRuntime = __napiModule.exports.shutdownAsyncRuntime
module.exports.startAsyncRuntime = __napiModule.exports.startAsyncRuntime
