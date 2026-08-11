//+------------------------------------------------------------------+
//|                  PivotRadar_Hybrid_IntraVela_ema50_D5.mq5        |
//|         Radar de Microcontextos — EURUSD                         |
//|         TODOS LOS DETECTORES INTRAVELA (M15)                    |
//|         HIPÓTESIS DE ANTICIPACIÓN PARA OPCIONES BINARIAS        |
//|         CÓDIGO COMPLETO CORREGIDO v7.6                          |
//+------------------------------------------------------------------+
#property copyright "PivotRadar XAUUSD | Calibrado: 2026-07-24 20:01"
#property version   "7.70"
#property strict

//+------------------------------------------------------------------+
//| CALIBRACION AUTOMATICA -- XAUUSD
//| Generado: 2026-07-24 20:01
//| Oro, 2 decimales, spreads amplios, movimientos de 10-30 pips/dolar. Requiere umbrales altos.
//| EMA Fast: 21 | EMA Slow: 50
//| ATR Compresion: 0.7 | Expansion: 1.5
//| Probabilidades: D1=60% D2=65% D3=60% D4=60% D5=70%
//+------------------------------------------------------------------+
//+------------------------------------------------------------------+
//| INPUTS                                                          |
//+------------------------------------------------------------------+
input string   InpNtfyTopic   = "";
input string   InpNtfyServer  = "https://ntfy.sh";
input int      InpTimerSec    = 1;
input bool     InpModoTest    = false;

//--- D1 (Ruptura de rango) - INTRAVELA
input int      InpN_Ruptura   = 3;
input double   InpD1_ATRThreshold = 0.8;
input double   InpBodyRatio_Min = 0.5;
input bool     InpD1_UseRetest = true;
input bool     InpD1_UseVolume = true;
input double   InpD1_MinVolume = 1.5;

//--- D2 (Liquidity Sweep + Reclaim) - INTRAVELA
input int      InpSweep_N     = 6;
input double   InpSweepWickMin = 0.65;
input double   InpReclaimBodyMin = 0.65;
input int      InpEqualHL_Window = 8;
input double   InpEqualHL_Tol = 0.25;
input bool     InpD2_Anticipar = true;

//--- D3 (Fair Value Gap) - INTRAVELA
input double   InpFVG_MinSizeATR = 0.4;
input double   InpFVG_BodyRatio = 0.65;
input double   InpFVG_MitigUmbral = 0.6;

//--- D4 (Order Block) - INTRAVELA
input int      InpOB_Lookback = 8;
input double   InpOB_BodyMin = 0.5;
input double   InpOB_ImpulseMin = 1.2;

//--- D5 (MSS H4 + Sweep) - INTRAVELA
input int      InpMSS_LookbackH4 = 12;
input int      InpMSS_MaxAgeH4Bars = 8;

//--- D0 (Estructura)
input int      InpPivotDepth = 2;
input int      InpPivotLookback = 24;
input double   InpSweepDistancia = 2.0;
input double   InpZonaMargen = 0.8;
input double   InpPesoEstructural = 0.25;

//--- Persistencia
input string   InpColaSenalesFile = "Cola_Senales_v77.csv";
input int      InpLockTimeoutMs = 5000;
input int      InpLockStaleSec = 5;
input bool     InpColaD1_Enabled = true;
input bool     InpColaD2_Enabled = true;
input bool     InpColaD3_Enabled = true;
input bool     InpColaD4_Enabled = true;
input bool     InpColaD5_Enabled = true;
input int      InpCsvFlushSec = 30;

//+------------------------------------------------------------------+
//| CONSTANTES                                                       |
//+------------------------------------------------------------------+
#define ATR_BUFFER_SIZE    55
#define MAX_PENDING_SIGNALS 500
#define MAX_ALERT_QUEUE    50
#define BS_MIN_RUPTURA     0.30

//+------------------------------------------------------------------+
//| ESTRUCTURA D0                                                    |
//+------------------------------------------------------------------+
struct EstructuraRef
{
   datetime timestamp;
   double   swing_high;
   double   swing_low;
   double   swing_high_ant;
   double   swing_low_ant;
   double   sweep_nivel;
   int      sweep_dir;
   double   zona_alta;
   double   zona_baja;
   bool     en_zona;
   string   dir_estructura;
   bool     valida;

   EstructuraRef()
   {
      timestamp = 0;
      swing_high = 0;
      swing_low = 0;
      swing_high_ant = 0;
      swing_low_ant = 0;
      sweep_nivel = 0;
      sweep_dir = 0;
      zona_alta = 0;
      zona_baja = 0;
      en_zona = false;
      dir_estructura = "NEUTRO";
      valida = false;
   }
};

//+------------------------------------------------------------------+
//| ESTRUCTURA DE SEÑAL                                              |
//+------------------------------------------------------------------+
struct Signal
{
   ulong    id;
   datetime entry_time;
   int      entry_bar_shift;
   string   symbol;
   int      direction;
   double   entry_price;
   string   detector;
   string   tipo;

   double   cr, bs, bs_pips, br;
   double   range_break_pips;
   double   nivel_estructural;

   double   sweep_wick_ratio;
   double   sweep_volume_ratio;
   double   reclaim_body_ratio;
   int      sweep_bars_ago;
   bool     equal_hl_detected;
   double   level_swept;

   double   fvg_size_pips, fvg_size_atr;
   bool     fvg_mitigated;
   double   fvg_top, fvg_bottom;

   double   ob_high, ob_low;
   int      ob_bars_ago;
   double   ob_impulse_atr;
   bool     ob_confluence;

   bool     mss_aligned;
   int      mss_bars_ago_h4;
   string   mss_direction;
   double   mss_level;

   double   atr14, spread_pips, volume_ratio;
   string   session, kill_zone, trend_d1;
   bool     vol_expanding, vol_compressing;

   double   calidad_sweep;
   double   calidad_mss;
   double   calidad_fvg;
   double   calidad_ob;
   double   salud_tendencial;

   double   g1_compresion;
   double   g2_persistencia;
   double   g3_eficiencia;
   double   g4_agotamiento;

   double   conf_sweep_fvg;
   double   conf_completa;

   bool     es_intravela;

   double   contexto_estructural;
   string   estructura_direccion;
   double   distancia_al_sweep;
   bool     en_zona_estructural;

   string   hipotesis_causa;
   string   hipotesis_efecto;
   string   hipotesis_razon;
   string   hipotesis_invalidez;
   int      hipotesis_expiry_velas;
   int      hipotesis_expiry_minutos;
   int      hipotesis_prob_min;
   int      hipotesis_prob_max;
   string   hipotesis_zona;
   double   hipotesis_objetivo;              // <--- NUEVO
   string   hipotesis_texto;

   int      signal_age_bars;
   bool     measured[4];
   double   retorno[4];
   double   mfe[4];
   double   mae[4];
   bool     gap_detected;
   bool     completada;

   Signal()
   {
      id = 0;
      entry_time = 0;
      entry_bar_shift = -1;
      symbol = "";
      direction = 0;
      entry_price = 0.0;
      detector = "";
      tipo = "";
      cr = bs = bs_pips = br = 0.0;
      range_break_pips = 0.0;
      nivel_estructural = 0.0;
      sweep_wick_ratio = sweep_volume_ratio = reclaim_body_ratio = 0.0;
      sweep_bars_ago = 0;
      equal_hl_detected = false;
      level_swept = 0.0;
      fvg_size_pips = fvg_size_atr = 0.0;
      fvg_mitigated = false;
      fvg_top = fvg_bottom = 0.0;
      ob_high = ob_low = 0.0;
      ob_bars_ago = 0;
      ob_impulse_atr = 0.0;
      ob_confluence = false;
      mss_aligned = false;
      mss_bars_ago_h4 = 0;
      mss_direction = "";
      mss_level = 0.0;
      atr14 = spread_pips = volume_ratio = 0.0;
      session = kill_zone = trend_d1 = "";
      vol_expanding = vol_compressing = false;
      calidad_sweep = calidad_mss = calidad_fvg = calidad_ob = 0.0;
      salud_tendencial = 0.0;
      g1_compresion = g2_persistencia = g3_eficiencia = g4_agotamiento = 0.0;
      conf_sweep_fvg = conf_completa = 0.0;
      es_intravela = true;
      contexto_estructural = 0.0;
      estructura_direccion = "NEUTRO";
      distancia_al_sweep = 0.0;
      en_zona_estructural = false;
      hipotesis_causa = "";
      hipotesis_efecto = "";
      hipotesis_razon = "";
      hipotesis_invalidez = "";
      hipotesis_expiry_velas = 0;
      hipotesis_expiry_minutos = 0;
      hipotesis_prob_min = 0;
      hipotesis_prob_max = 0;
      hipotesis_zona = "NEUTRO";
      hipotesis_objetivo = 0.0;
      hipotesis_texto = "";
      signal_age_bars = 0;
      ArrayInitialize(measured, false);
      ArrayInitialize(retorno, 0.0);
      ArrayInitialize(mfe, 0.0);
      ArrayInitialize(mae, 0.0);
      gap_detected = false;
      completada = false;
   }
};

//+------------------------------------------------------------------+
//| ESTRUCTURA DE ALERTA                                             |
//+------------------------------------------------------------------+
struct AlertEntry
{
   string   text;
   int      retry_count;
   datetime last_retry;
   datetime created_at;
   string   content_hash;
};

//+------------------------------------------------------------------+
//| ESTRUCTURA DE CACHE MSS_H4                                       |
//+------------------------------------------------------------------+
struct MSSCache
{
   bool     valid;
   datetime calc_time;
   int      bars_ago;
   string   dir;
   double   level;
};

//+------------------------------------------------------------------+
//| ESTRUCTURA DE CACHE ZONA                                         |
//+------------------------------------------------------------------+
struct ZonaCache
{
   bool     valid;
   datetime calc_time;
   double   mid;
};

//+------------------------------------------------------------------+
//| DECLARACIONES DE FUNCIONES                                       |
//+------------------------------------------------------------------+
bool   AcquireLock();
void   ReleaseLock();
ulong  BuildSignalId(datetime bar_time, string detector, int direction, double keyLevel);
double GetVolumeRatio(int bar_shift, int n_lookback);
double GetVolumeRatioCached(int bar_shift, int n_lookback);
double CalcularG1_Compresion();
double CalcularG2_Persistencia();
double CalcularG3_Eficiencia();
double CalcularG4_Agotamiento();
double CalcularCalidadSweep(double wick, double reclaim, double vol, int bars_ago, bool equal_hl);
double CalcularCalidadMSS(double wick, double reclaim, int mss_bars_ago);
double CalcularCalidadFVG(double fvg_size, double br_impulso, bool defendido);
double CalcularCalidadOB(double impulso, int ob_bars, double vol);
double CalcularSaludTendencial(int trend_velas, double slope, string trend_d1, int dir);
bool   HuboSenalRecienteEnDireccion(string det, int dir, int n_velas);
double CalcularConfluenciaSweepFVG(int dir, bool fvg_ahora, double fvg_size);
double CalcularConfluenciaCompleta(int dir, bool fvg_ahora, double fvg_size);
void   AgregarCandidata(Signal &signal);
void   ResolverConfluenciasYRutear();
void   RouteSignal(Signal &signal);
bool   IsDuplicateSignal(ulong id);
void   WriteSignalToCSV(const Signal &signal);
void   LogError(string msg);
void   SavePendingSignals();
void   LoadPendingSignals();
void   MeasureReturns();
void   ActualizarEstructura();
void   DetectarPivotsH1();
void   IdentificarSweepMaestro();
void   DefinirZonaDeInteres();
void   DeterminarDireccionEstructural();
double EvaluarContextoEstructural(int dir, double nivel, string det, string trend_d1, double &dist);
void   GenerarHipotesis(Signal &sig);
int    CalcularVencimiento(const Signal &sig);
void   MotorD1_IntraBar(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
void   MotorD2_LiquiditySweep(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
void   MotorD2_Anticipacion(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
void   MotorD3_IntraBar(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
void   MotorD4_OrderBlockConfluence(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
void   MotorD5_MSS_Sweep(double vol, string session, string kill_zone, bool vol_exp, bool vol_comp, string trend_d1);
string ClasificarD1(double br, double bs, string session);
string ClasificarD2(double wick, double vol, double reclaim, bool equal_hl);
string ClasificarD2_Anticipacion(double wick, double vol, int confluencias);
string ClasificarD3(double fvg_size, double br, int trend_velas, double slope);
string ClasificarD4(double impulso, double vol, int ob_bars);
string ClasificarD5(int mss_bars, double wick, double reclaim, string kill_zone);
double Clamp01100(double v);
void   HandleCopyBufferFail(string name, int copied);
bool   UpdateIndicators();
double GetSpreadPips();
string GetSession(datetime bar_time);
string GetKillZone(datetime bar_time);
string GetTrendD1(const double &ema50_d1[], const double &ema200_d1[]);
int    GetTrendVelas();
bool   IsVolatilityExpanding();
bool   IsVolatilityCompressing();
bool   DetectMSS_H4(int &bars_ago, string &dir, double &level);
int    CheckLoadHistory(string symbol, ENUM_TIMEFRAMES period, int min_bars);
void   BuildAlertText(const Signal &signal, string &msg);
bool   SendNtfyMessage(string text);
void   QueueAlert(string text);
void   ProcessAlertQueue();
void   FlushAlertQueue();
void   FlushCSVBuffer();
void   TestNtfy();
void   ProcessIntraBar();
bool   EsZonaPremiumDiscount(double nivel, string &zona);

//+------------------------------------------------------------------+
//| VARIABLES GLOBALES                                               |
//+------------------------------------------------------------------+
datetime g_lastBarTime = 0;
datetime g_lastAlertTime = 0;
datetime g_lastNtfyTime = 0;
Signal   g_pending_signals[];
Signal   g_candidatas_vela[];
AlertEntry g_alert_queue[];
int      g_copybuffer_fail_count = 0;

int g_handle_atr14;
int g_handle_ema21, g_handle_ema50;
int g_handle_ema50_d1, g_handle_ema200_d1;

double g_atr14_buffer[];
double g_ema21_buffer[];
double g_ema50_buffer[];
double g_ema50_d1_buffer[];
double g_ema200_d1_buffer[];

string g_csv_filename, g_pending_filename, g_log_filename, g_lock_filename;
string g_volume_source = "TICK_PROXY";
datetime g_lastVolumeCalcTime = 0;
datetime g_processing_start_time = 0;
string   g_csv_buffer = "";
datetime g_csv_last_flush = 0;
datetime g_lastG_calcBar = 0;
double g_cachedVolumeRatio = 1.0;
int    g_cachedVolumeBarShift = -1;
int    g_cachedVolumeLookback = -1;
double g_atr14_history[20];

bool g_isProcessing = false;

struct DetectorLatch
{
   datetime lastSignalBar;
   string   lastPatternKey;
   bool     hasFiredThisBar;
};
DetectorLatch g_detectorLatch[6];

double g_g1_compresion = 0.0;
double g_g2_persistencia = 0.0;
double g_g3_eficiencia = 0.0;
double g_g4_agotamiento = 0.0;

EstructuraRef g_estructura;
datetime g_lastStructUpdate = 0;

MSSCache g_mss_cache;
ZonaCache g_zona_cache;

//+------------------------------------------------------------------+
//| FUNCIÓN AUXILIAR: INICIALIZAR LATCHES                            |
//+------------------------------------------------------------------+
void InitLatches()
{
   for(int i = 0; i < 6; i++)
   {
      g_detectorLatch[i].lastSignalBar = 0;
      g_detectorLatch[i].lastPatternKey = "";
      g_detectorLatch[i].hasFiredThisBar = false;
   }
}

//+------------------------------------------------------------------+
//| CHECKLOADHISTORY                                                 |
//+------------------------------------------------------------------+
int CheckLoadHistory(string symbol, ENUM_TIMEFRAMES period, int min_bars)
{
   if(!SymbolInfoInteger(symbol, SYMBOL_SELECT))
      SymbolSelect(symbol, true);

   datetime first_date;
   SeriesInfoInteger(symbol, period, SERIES_FIRSTDATE, first_date);
   if(first_date > 0)
   {
      int available = iBars(symbol, period);
      if(available >= min_bars) return 1;
   }

   datetime start_date = TimeCurrent() - (min_bars * PeriodSeconds(period) * 2);
   datetime times[1];
   int fail_cnt = 0;

   while(!IsStopped() && fail_cnt < 5)
   {
      if(CopyTime(symbol, period, start_date, 1, times) > 0)
      {
         int available = iBars(symbol, period);
         if(available >= min_bars) return 0;
      }
      int err = GetLastError();
      if(err == 0) break;
      if(err == 4302 || err == 4401)
      {
         Sleep(50);
         continue;
      }
      fail_cnt++;
      Sleep(50);
   }
   return -5;
}

//+------------------------------------------------------------------+
//| ONINIT                                                           |
//+------------------------------------------------------------------+
int OnInit()
{
   if(Period() != PERIOD_M15)
   {
      Print("ERROR: EA debe adjuntarse a gráfico M15. Periodo actual: ", EnumToString(Period()));
      return INIT_FAILED;
   }

   if(!SymbolInfoInteger(_Symbol, SYMBOL_SELECT))
      SymbolSelect(_Symbol, true);

   int bars_d1 = iBars(_Symbol, PERIOD_D1);
   if(bars_d1 < 200)
   {
      Print("Forzando carga de datos D1...");
      CheckLoadHistory(_Symbol, PERIOD_D1, 250);
      bars_d1 = iBars(_Symbol, PERIOD_D1);
   }
   CheckLoadHistory(_Symbol, PERIOD_H4, 50);
   CheckLoadHistory(_Symbol, PERIOD_H1, 50);

   g_handle_atr14 = iATR(_Symbol, PERIOD_M15, 14);
   g_handle_ema21 = iMA(_Symbol, PERIOD_M15, 21, 0, MODE_EMA, PRICE_CLOSE);
   g_handle_ema50 = iMA(_Symbol, PERIOD_M15, 50, 0, MODE_EMA, PRICE_CLOSE);
   g_handle_ema50_d1 = iMA(_Symbol, PERIOD_D1, 50, 0, MODE_EMA, PRICE_CLOSE);
   g_handle_ema200_d1 = iMA(_Symbol, PERIOD_D1, 200, 0, MODE_EMA, PRICE_CLOSE);

   if(g_handle_atr14 == INVALID_HANDLE || g_handle_ema21 == INVALID_HANDLE ||
      g_handle_ema50 == INVALID_HANDLE ||
      g_handle_ema50_d1 == INVALID_HANDLE || g_handle_ema200_d1 == INVALID_HANDLE)
   {
      Print("ERROR: Fallo la creación de handles.");
      return INIT_FAILED;
   }

   ArraySetAsSeries(g_atr14_buffer, true);
   ArraySetAsSeries(g_ema21_buffer, true);
   ArraySetAsSeries(g_ema50_buffer, true);
   ArraySetAsSeries(g_ema50_d1_buffer, true);
   ArraySetAsSeries(g_ema200_d1_buffer, true);

   ArrayResize(g_atr14_buffer, ATR_BUFFER_SIZE);
   ArrayResize(g_ema21_buffer, ATR_BUFFER_SIZE);
   ArrayResize(g_ema50_buffer, ATR_BUFFER_SIZE);
   ArrayResize(g_ema50_d1_buffer, 5);
   ArrayResize(g_ema200_d1_buffer, 5);

   InitLatches();

   g_mss_cache.valid = false;
   g_mss_cache.calc_time = 0;
   g_zona_cache.valid = false;
   g_zona_cache.calc_time = 0;

   string suffix = "_v77_" + _Symbol;
   g_csv_filename = "Micro_v7" + suffix + ".csv";
   g_pending_filename = "Pending_v7" + suffix + ".csv";
   g_log_filename = "Errores_v7_" + TimeToString(TimeCurrent(), TIME_DATE) + suffix + ".log";
   g_lock_filename = InpColaSenalesFile + ".lock";

   ArrayResize(g_alert_queue, 0);
   ArrayResize(g_pending_signals, 0);
   ArrayInitialize(g_atr14_history, 0.0);
   // Precargar con ATR actual para evitar ceros en primeras 20 barras
   double atr_init[];
   if(CopyBuffer(g_handle_atr14, 0, 0, 1, atr_init) > 0 && atr_init[0] > 0)
   {
      for(int i_hist = 0; i_hist < 20; i_hist++)
         g_atr14_history[i_hist] = atr_init[0];
   }

   if(bars_d1 >= 50)
   {
      CopyBuffer(g_handle_ema50_d1, 0, 0, 5, g_ema50_d1_buffer);
      CopyBuffer(g_handle_ema200_d1, 0, 0, 5, g_ema200_d1_buffer);
   }

   LoadPendingSignals();
   g_lastBarTime = iTime(_Symbol, PERIOD_M15, 0);
   g_csv_last_flush = TimeCurrent();
   EventSetTimer(InpTimerSec);
   ActualizarEstructura();

   Print("=== MODO COMPARACION: v7.7 aislado de v7.6 ===");
   Print("=== PivotRadar Hybrid v7.6 ===");
   Print("Símbolo: ", _Symbol, " | Timeframe: M15");
   Print("TODOS LOS DETECTORES INTRAVELA");
   Print("HIPÓTESIS DE ANTICIPACIÓN para Opciones Binarias");
   Print("=================================================");

   if(InpModoTest) TestNtfy();

   return INIT_SUCCEEDED;
}

//+------------------------------------------------------------------+
//| ONDEINIT                                                         |
//+------------------------------------------------------------------+
void OnDeinit(const int reason)
{
   SavePendingSignals();
   FlushAlertQueue();
   IndicatorRelease(g_handle_atr14);
   IndicatorRelease(g_handle_ema21);
   IndicatorRelease(g_handle_ema50);
   IndicatorRelease(g_handle_ema50_d1);
   IndicatorRelease(g_handle_ema200_d1);
   FlushCSVBuffer();
   EventKillTimer();
   Print("PivotRadar Hybrid v7.6 finalizado.");
}

//+------------------------------------------------------------------+
//| ONTICK                                                           |
//+------------------------------------------------------------------+
void OnTick()
{
   if(g_isProcessing)
   {
      if(TimeCurrent() - g_processing_start_time > 10)
      {
         Print("WARNING: g_isProcessing stale >10s, forcing reset");
         g_isProcessing = false;
      }
      else
      {
         return;
      }
   }
   g_isProcessing = true;
   g_processing_start_time = TimeCurrent();

   datetime currentBar = iTime(_Symbol, PERIOD_M15, 0);
   static ulong lastProcessTick = 0;
   ulong now_tick = GetTickCount64();

   if(now_tick - lastProcessTick < 200)
   {
      g_isProcessing = false;
      return;
   }
   lastProcessTick = now_tick;

   ProcessIntraBar();
   ProcessAlertQueue();
   g_isProcessing = false;
}

//+------------------------------------------------------------------+
//| ONTIMER                                                          |
//+------------------------------------------------------------------+
void OnTimer()
{
   if(g_isProcessing)
   {
      if(TimeCurrent() - g_processing_start_time > 10)
      {
         Print("WARNING: g_isProcessing stale >10s (timer), forcing reset");
         g_isProcessing = false;
      }
      else
      {
         return;
      }
   }
   g_isProcessing = true;
   g_processing_start_time = TimeCurrent();

   SavePendingSignals();
   FlushCSVBuffer();
   ProcessAlertQueue();
   g_isProcessing = false;
}

//+------------------------------------------------------------------+
//| UPDATEINDICATORS                                                 |
//+------------------------------------------------------------------+
bool UpdateIndicators()
{
   int copied;

   copied = CopyBuffer(g_handle_atr14, 0, 0, ATR_BUFFER_SIZE, g_atr14_buffer);
   if(copied < 3) { HandleCopyBufferFail("ATR14", copied); return false; }
   copied = CopyBuffer(g_handle_ema21, 0, 0, ATR_BUFFER_SIZE, g_ema21_buffer);
   if(copied < 3) { HandleCopyBufferFail("EMA21", copied); return false; }
   copied = CopyBuffer(g_handle_ema50, 0, 0, ATR_BUFFER_SIZE, g_ema50_buffer);
   if(copied < 3) { HandleCopyBufferFail("EMA50", copied); return false; }

   copied = CopyBuffer(g_handle_ema50_d1, 0, 0, 5, g_ema50_d1_buffer);
   if(copied < 2) { HandleCopyBufferFail("EMA50_D1", copied); return false; }
   copied = CopyBuffer(g_handle_ema200_d1, 0, 0, 5, g_ema200_d1_buffer);
   if(copied < 2) { HandleCopyBufferFail("EMA200_D1", copied); return false; }

   g_copybuffer_fail_count = 0;
   return true;
}

void HandleCopyBufferFail(string name, int copied)
{
   g_copybuffer_fail_count++;
   Print("CopyBuffer falló: ", name, " copiados=", copied, " fallos=", g_copybuffer_fail_count);
}

//+------------------------------------------------------------------+
//| GETTRENDD1                                                       |
//+------------------------------------------------------------------+
string GetTrendD1(const double &ema50_d1[], const double &ema200_d1[])
{
   if(ArraySize(ema50_d1) < 2 || ArraySize(ema200_d1) < 2) return "NEUTRO";
   // Índice [1] = barra cerrada, evita repintado de vela en formación
   double ema50 = ema50_d1[1];
   double ema200 = ema200_d1[1];
   if(ema50 == 0 || ema200 == 0) return "NEUTRO";
   double eps = ema200 * 0.0005;
   if(ema50 > ema200 + eps) return "ALCISTA";
   if(ema50 < ema200 - eps) return "BAJISTA";
   return "NEUTRO";
}

//+------------------------------------------------------------------+
//| GETSPREADPIPS                                                    |
//+------------------------------------------------------------------+
double GetSpreadPips()
{
   long spread = SymbolInfoInteger(_Symbol, SYMBOL_SPREAD);
   if(spread > 0) return (double)spread;
   double ask = SymbolInfoDouble(_Symbol, SYMBOL_ASK);
   double bid = SymbolInfoDouble(_Symbol, SYMBOL_BID);
   if(ask > 0 && bid > 0 && ask > bid) return (ask - bid) / _Point;
   return 0.0;
}

//+------------------------------------------------------------------+
//| GETVOLUMERATIO                                                   |
//+------------------------------------------------------------------+
double GetVolumeRatio(int bar_shift, int n_lookback)
{
   long vol_signal = iVolume(_Symbol, PERIOD_M15, bar_shift);
   if(vol_signal <= 0) return 1.0;
   long sum_prev = 0;
   int count = 0;
   for(int i = 1; i <= n_lookback; i++)
   {
      long vol = iVolume(_Symbol, PERIOD_M15, bar_shift + i);
      if(vol > 0) { sum_prev += vol; count++; }
   }
   if(count == 0 || sum_prev <= 0) return 1.0;
   return (double)vol_signal / ((double)sum_prev / count);
}

//+------------------------------------------------------------------+
//| GETVOLUMERATIOCACHED                                             |
//+------------------------------------------------------------------+
double GetVolumeRatioCached(int bar_shift, int n_lookback)
{
   datetime bar_time = iTime(_Symbol, PERIOD_M15, bar_shift);
   bool valid = (g_lastVolumeCalcTime == bar_time &&
                 g_cachedVolumeBarShift == bar_shift &&
                 g_cachedVolumeLookback == n_lookback);

   if(bar_shift == 0 && valid && (TimeCurrent() - g_lastVolumeCalcTime > 1))
      valid = false;

   if(valid) return g_cachedVolumeRatio;

   double result = GetVolumeRatio(bar_shift, n_lookback);
   g_cachedVolumeRatio = result;
   g_cachedVolumeBarShift = bar_shift;
   g_cachedVolumeLookback = n_lookback;
   g_lastVolumeCalcTime = bar_time;
   return result;
}

//+------------------------------------------------------------------+
//| GETSESSION                                                       |
//+------------------------------------------------------------------+
string GetSession(datetime bar_time)
{
   MqlDateTime dt;
   TimeToStruct(bar_time, dt);
   int hour = dt.hour;
   if(hour >= 0 && hour < 7) return "ASIA";
   if(hour >= 7 && hour < 13) return "LONDON";
   if(hour >= 13 && hour < 15) return "NY_OPEN";
   if(hour >= 15 && hour < 16) return "LONDON_CLOSE";
   if(hour >= 16 && hour < 21) return "NY";
   return "OUT";
}

//+------------------------------------------------------------------+
//| GETKILLZONE                                                      |
//+------------------------------------------------------------------+
string GetKillZone(datetime bar_time)
{
   MqlDateTime dt;
   TimeToStruct(bar_time, dt);
   int hour = dt.hour, min = dt.min;
   if(hour == 7 || hour == 8) return "LONDON_OPEN_KILL";
   if(hour == 13 || (hour == 14 && min <= 30)) return "NY_OPEN_KILL";
   if(hour >= 13 && hour < 15) return "LONDON_NY_OVERLAP";
   return "NONE";
}

//+------------------------------------------------------------------+
//| GETTRENDVELAS                                                    |
//+------------------------------------------------------------------+
int GetTrendVelas()
{
   if(ArraySize(g_ema21_buffer) < 2 || ArraySize(g_ema50_buffer) < 2) return 0;
   bool up = (g_ema21_buffer[1] > g_ema50_buffer[1]);
   bool down = (g_ema21_buffer[1] < g_ema50_buffer[1]);
   if(!up && !down) return 0;
   int count = 0;
   int max_i = MathMin(ATR_BUFFER_SIZE, MathMin(ArraySize(g_ema21_buffer), ArraySize(g_ema50_buffer)));
   for(int i = 1; i < max_i; i++)
   {
      bool u = (g_ema21_buffer[i] > g_ema50_buffer[i]);
      bool d = (g_ema21_buffer[i] < g_ema50_buffer[i]);
      if(!u && !d) continue;
      if(up && !u) break;
      if(down && !d) break;
      count++;
   }
   return count;
}

//+------------------------------------------------------------------+
//| ISVOLATILITYEXPANDING                                            |
//+------------------------------------------------------------------+
bool IsVolatilityExpanding()
{
   if(g_atr14_history[10] == 0) return false;
   double avg = 0;
   int count = 0;
   for(int i = 1; i <= 10; i++)
   {
      if(g_atr14_history[i] > 0)
      {
         avg += g_atr14_history[i];
         count++;
      }
   }
   if(count == 0) return false;
   avg /= count;
   return (g_atr14_history[0] > avg * 1.30);
}

//+------------------------------------------------------------------+
//| ISVOLATILITYCOMPRESSING                                          |
//+------------------------------------------------------------------+
bool IsVolatilityCompressing()
{
   if(g_atr14_history[10] == 0) return false;
   double avg = 0;
   int count = 0;
   for(int i = 1; i <= 10; i++)
   {
      if(g_atr14_history[i] > 0)
      {
         avg += g_atr14_history[i];
         count++;
      }
   }
   if(count == 0) return false;
   avg /= count;
   return (g_atr14_history[0] < avg * 0.80);
}

//+------------------------------------------------------------------+
//| UPDATEATRHISTORY                                                 |
//+------------------------------------------------------------------+
void UpdateATRHistory()
{
   for(int i = 19; i > 0; i--)
      g_atr14_history[i] = g_atr14_history[i-1];
   g_atr14_history[0] = g_atr14_buffer[0];
}

//+------------------------------------------------------------------+
//| CLAMP01100                                                       |
//+------------------------------------------------------------------+
double Clamp01100(double v)
{
   if(v < 0) return 0;
   if(v > 100) return 100;
   return v;
}

//+------------------------------------------------------------------+
//| BUILDPATTERNKEY                                                  |
//+------------------------------------------------------------------+
string BuildPatternKey(string detector, int direction, double keyLevel)
{
   // FIX: keyLevel eliminado del hash para evitar múltiples disparos por fluctuación de precio
   return detector + "|" + IntegerToString(direction);
}

//+------------------------------------------------------------------+
//| HASDETECTORFIREDTHISBAR                                          |
//+------------------------------------------------------------------+
bool HasDetectorFiredThisBar(string detector, int direction, double keyLevel)
{
   int idx = -1;
   if(detector == "D1") idx = 0;
   else if(detector == "D2") idx = 1;
   else if(detector == "D3") idx = 2;
   else if(detector == "D3_DEF") idx = 3;
   else if(detector == "D4") idx = 4;
   else if(detector == "D5") idx = 5;
   if(idx < 0) return false;

   datetime currentBar = iTime(_Symbol, PERIOD_M15, 0);
   string patternKey = BuildPatternKey(detector, direction, keyLevel);

   if(g_detectorLatch[idx].lastSignalBar != currentBar)
   {
      g_detectorLatch[idx].hasFiredThisBar = false;
      g_detectorLatch[idx].lastPatternKey = "";
   }

   if(g_detectorLatch[idx].hasFiredThisBar && g_detectorLatch[idx].lastPatternKey == patternKey)
      return true;

   return false;
}

//+------------------------------------------------------------------+
//| MARKDETECTORFIRED                                                |
//+------------------------------------------------------------------+
void MarkDetectorFired(string detector, int direction, double keyLevel)
{
   int idx = -1;
   if(detector == "D1") idx = 0;
   else if(detector == "D2") idx = 1;
   else if(detector == "D3") idx = 2;
   else if(detector == "D3_DEF") idx = 3;
   else if(detector == "D4") idx = 4;
   else if(detector == "D5") idx = 5;
   if(idx < 0) return;

   datetime currentBar = iTime(_Symbol, PERIOD_M15, 0);
   g_detectorLatch[idx].lastSignalBar = currentBar;
   g_detectorLatch[idx].lastPatternKey = BuildPatternKey(detector, direction, keyLevel);
   g_detectorLatch[idx].hasFiredThisBar = true;
}

//+------------------------------------------------------------------+
//| BUILDSIGNALID                                                    |
//+------------------------------------------------------------------+
ulong BuildSignalId(datetime bar_time, string detector, int direction, double keyLevel)
{
   ulong hash = (ulong)bar_time;
   for(int i = 0; i < StringLen(detector); i++)
      hash = hash * 31 + (ulong)StringGetCharacter(detector, i);
   hash = hash * 31 + (ulong)(direction + 2);
   // FIX: keyLevel se hashea de forma estable para evitar colisiones por micro-fluctuaciones
   hash = hash * 31 + (ulong)(NormalizeDouble(keyLevel, _Digits) * 100000.0);
   return hash;
}

//+------------------------------------------------------------------+
//| ISDUPLICATESIGNAL                                                |
//+------------------------------------------------------------------+
bool IsDuplicateSignal(ulong id)
{
   for(int i = 0; i < ArraySize(g_pending_signals); i++)
   {
      if(g_pending_signals[i].id == id) return true;
   }
   return false;
}

//+------------------------------------------------------------------+
//| AGREGARCANDIDATA                                                 |
//+------------------------------------------------------------------+
void AgregarCandidata(Signal &signal)
{
   int size = ArraySize(g_candidatas_vela);
   ArrayResize(g_candidatas_vela, size + 1);
   g_candidatas_vela[size] = signal;
}

//+------------------------------------------------------------------+
//| D0 — ESTRUCTURA                                                  |
//+------------------------------------------------------------------+
void ActualizarEstructura()
{
   DetectarPivotsH1();
   IdentificarSweepMaestro();
   DefinirZonaDeInteres();
   DeterminarDireccionEstructural();
   g_estructura.valida = (g_estructura.swing_high > 0 ||
                          g_estructura.swing_low > 0 ||
                          g_estructura.sweep_nivel > 0);
   g_estructura.timestamp = TimeCurrent();
}

//+------------------------------------------------------------------+
//| DETECTARPIVOTSH1                                                 |
//+------------------------------------------------------------------+
void DetectarPivotsH1()
{
   g_estructura.swing_high = 0;
   g_estructura.swing_low = 0;
   g_estructura.swing_high_ant = 0;
   g_estructura.swing_low_ant = 0;

   int depth = InpPivotDepth;
   int lookback = InpPivotLookback;
   int start = depth + 1;
   int end = lookback - depth - 1;

   double highs[];
   double lows[];
   int max_pivots = 50;
   ArrayResize(highs, max_pivots);
   ArrayResize(lows, max_pivots);
   int high_count = 0, low_count = 0;

   for(int i = start; i < end && i < max_pivots; i++)
   {
      double high_i = iHigh(_Symbol, PERIOD_H1, i);
      if(high_i == 0) continue;
      bool is_swing = true;
      for(int j = 1; j <= depth; j++)
      {
         if(iHigh(_Symbol, PERIOD_H1, i-j) >= high_i ||
            iHigh(_Symbol, PERIOD_H1, i+j) >= high_i)
         {
            is_swing = false;
            break;
         }
      }
      if(is_swing && high_count < max_pivots)
      {
         highs[high_count++] = high_i;
      }
   }

   for(int i = start; i < end && i < max_pivots; i++)
   {
      double low_i = iLow(_Symbol, PERIOD_H1, i);
      if(low_i == 0) continue;
      bool is_swing = true;
      for(int j = 1; j <= depth; j++)
      {
         if(iLow(_Symbol, PERIOD_H1, i-j) <= low_i ||
            iLow(_Symbol, PERIOD_H1, i+j) <= low_i)
         {
            is_swing = false;
            break;
         }
      }
      if(is_swing && low_count < max_pivots)
      {
         lows[low_count++] = low_i;
      }
   }

   if(high_count > 0)
   {
      double max_high = highs[0];
      for(int i = 1; i < high_count; i++)
      {
         if(highs[i] > max_high) max_high = highs[i];
      }
      g_estructura.swing_high = max_high;

      double second_high = 0;
      for(int i = 0; i < high_count; i++)
      {
         if(highs[i] < max_high && highs[i] > second_high)
            second_high = highs[i];
      }
      g_estructura.swing_high_ant = second_high;
   }

   if(low_count > 0)
   {
      double min_low = lows[0];
      for(int i = 1; i < low_count; i++)
      {
         if(lows[i] < min_low) min_low = lows[i];
      }
      g_estructura.swing_low = min_low;

      double second_low = 999999.0;
      for(int i = 0; i < low_count; i++)
      {
         if(lows[i] > min_low && lows[i] < second_low)
            second_low = lows[i];
      }
      g_estructura.swing_low_ant = (second_low < 999999.0) ? second_low : 0;
   }
}

//+------------------------------------------------------------------+
//| IDENTIFICARSWEEPMAESTRO                                          |
//+------------------------------------------------------------------+
void IdentificarSweepMaestro()
{
   double price = iClose(_Symbol, PERIOD_M15, 0);
   double atr14 = g_atr14_buffer[0];
   double umbral = atr14 * InpSweepDistancia;

   g_estructura.sweep_nivel = 0;
   g_estructura.sweep_dir = 0;

   if(g_estructura.swing_high > 0 && MathAbs(price - g_estructura.swing_high) < umbral)
   {
      g_estructura.sweep_nivel = g_estructura.swing_high;
      g_estructura.sweep_dir = -1;
      return;
   }

   if(g_estructura.swing_low > 0 && MathAbs(price - g_estructura.swing_low) < umbral)
   {
      g_estructura.sweep_nivel = g_estructura.swing_low;
      g_estructura.sweep_dir = +1;
      return;
   }
}

//+------------------------------------------------------------------+
//| DEFINIRZONADEINTERES                                             |
//+------------------------------------------------------------------+
void DefinirZonaDeInteres()
{
   double price = iClose(_Symbol, PERIOD_M15, 0);
   double atr14 = g_atr14_buffer[0];
   double margen = atr14 * InpZonaMargen;

   if(g_estructura.swing_high > 0 && g_estructura.swing_low > 0)
   {
      g_estructura.zona_alta = MathMax(g_estructura.swing_high, g_estructura.swing_low) + margen;
      g_estructura.zona_baja = MathMin(g_estructura.swing_high, g_estructura.swing_low) - margen;
   }
   else if(g_estructura.swing_high > 0)
   {
      g_estructura.zona_alta = g_estructura.swing_high + margen;
      g_estructura.zona_baja = g_estructura.swing_high - margen;
   }
   else if(g_estructura.swing_low > 0)
   {
      g_estructura.zona_alta = g_estructura.swing_low + margen;
      g_estructura.zona_baja = g_estructura.swing_low - margen;
   }
   else
   {
      g_estructura.zona_alta = price + margen;
      g_estructura.zona_baja = price - margen;
   }

   g_estructura.en_zona = (price >= g_estructura.zona_baja && price <= g_estructura.zona_alta);
}

//+------------------------------------------------------------------+
//| DETERMINARDIRECCIONESTRUCTURAL                                   |
//+------------------------------------------------------------------+
void DeterminarDireccionEstructural()
{
   if(g_estructura.swing_high > 0 && g_estructura.swing_high_ant > 0 &&
      g_estructura.swing_low > 0 && g_estructura.swing_low_ant > 0)
   {
      bool hh = (g_estructura.swing_high > g_estructura.swing_high_ant);
      bool hl = (g_estructura.swing_low > g_estructura.swing_low_ant);
      if(hh && hl)
         g_estructura.dir_estructura = "ALCISTA";
      else if(!hh && !hl)
         g_estructura.dir_estructura = "BAJISTA";
      else
         g_estructura.dir_estructura = "NEUTRO";
   }
   else
      g_estructura.dir_estructura = "NEUTRO";
}

//+------------------------------------------------------------------+
//| ESZONAPREMIUMDISCOUNT                                            |
//+------------------------------------------------------------------+
bool EsZonaPremiumDiscount(double nivel, string &zona)
{
   datetime currentBar = iTime(_Symbol, PERIOD_M15, 0);

   if(g_zona_cache.valid && g_zona_cache.calc_time == currentBar)
   {
      double mid = g_zona_cache.mid;
      zona = (nivel > mid) ? "PREMIUM" : "DISCOUNT";
      return true;
   }

   double max_high = 0, min_low = 999999.0;
   for(int i = 1; i <= 50 && i < 100; i++)
   {
      double h = iHigh(_Symbol, PERIOD_M15, i);
      double l = iLow(_Symbol, PERIOD_M15, i);
      if(h == 0 || l == 0) break;
      if(h > max_high) max_high = h;
      if(l < min_low) min_low = l;
   }

   if(max_high > 0 && min_low > 0 && max_high > min_low)
   {
      double mid = (max_high + min_low) / 2.0;
      g_zona_cache.valid = true;
      g_zona_cache.calc_time = currentBar;
      g_zona_cache.mid = mid;
      zona = (nivel > mid) ? "PREMIUM" : "DISCOUNT";
      return true;
   }

   zona = "NEUTRO";
   return false;
}

//+------------------------------------------------------------------+
//| EVALUARCONTEXTOESTRUCTURAL                                       |
//+------------------------------------------------------------------+
double EvaluarContextoEstructural(int direction, double nivel, string detector, string trend_d1, double &distancia)
{
   double score = 0.0;
   distancia = 0.0;

   if(!g_estructura.valida || g_estructura.sweep_nivel == 0) return 50.0;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return 50.0;

   double tolerancia = atr14 * 0.5;
   distancia = MathAbs(nivel - g_estructura.sweep_nivel) / _Point;

   if(distancia <= tolerancia / _Point)
      score += 50.0;
   else if(distancia <= tolerancia * 2 / _Point)
      score += 30.0;
   else
      score += 10.0;

   if(g_estructura.en_zona) score += 25.0;

   if(g_estructura.dir_estructura != "NEUTRO")
   {
      if((direction == +1 && g_estructura.dir_estructura == "ALCISTA") ||
         (direction == -1 && g_estructura.dir_estructura == "BAJISTA"))
         score += 25.0;
      else
         score += 5.0;
   }
   else
      score += 10.0;

   return Clamp01100(score);
}

//+------------------------------------------------------------------+
//| CALCULARG1_COMPRESION                                            |
//+------------------------------------------------------------------+
double CalcularG1_Compresion()
{
   double atr_now = g_atr14_buffer[0];
   if(atr_now <= 0) return 50.0;

   double sum = 0;
   int count = 0;
   for(int i = 0; i < 20; i++)
   {
      if(g_atr14_history[i] > 0)
      {
         sum += g_atr14_history[i];
         count++;
      }
   }
   if(count == 0) return 50.0;

   double avg = sum / count;
   if(avg <= 0) return 50.0;

   return Clamp01100((1.5 - atr_now / avg) / 1.0 * 100.0);
}

//+------------------------------------------------------------------+
//| CALCULARG2_PERSISTENCIA                                          |
//+------------------------------------------------------------------+
double CalcularG2_Persistencia()
{
   int up10 = 0, down10 = 0, up20 = 0, down20 = 0;

   for(int i = 1; i <= 20; i++)
   {
      double ci = iClose(_Symbol, PERIOD_M15, i);
      double oi = iOpen(_Symbol, PERIOD_M15, i);
      bool up = (ci > oi);

      if(i <= 10)
      {
         if(up) up10++; else down10++;
      }
      if(up) up20++; else down20++;
   }

   double d10 = MathMax(up10, down10) / 10.0;
   double d20 = MathMax(up20, down20) / 20.0;

   return Clamp01100(Clamp01100((d10 - 0.5) / 0.5 * 100.0) * 0.6 +
                     Clamp01100((d20 - 0.5) / 0.5 * 100.0) * 0.4);
}

//+------------------------------------------------------------------+
//| CALCULARG3_EFICIENCIA                                            |
//+------------------------------------------------------------------+
double CalcularG3_Eficiencia()
{
   int n = 10;
   double ini = iClose(_Symbol, PERIOD_M15, n);
   double fin = iClose(_Symbol, PERIOD_M15, 0);
   double neto = MathAbs(fin - ini);
   double total = 0;

   for(int i = 0; i < n && i < 100; i++)
   {
      double h = iHigh(_Symbol, PERIOD_M15, i);
      double l = iLow(_Symbol, PERIOD_M15, i);
      if(h == 0 || l == 0) break;
      total += (h - l);
   }

   if(total <= 0) return 50.0;
   return Clamp01100(neto / total * 100.0);
}

//+------------------------------------------------------------------+
//| CALCULARG4_AGOTAMIENTO                                           |
//+------------------------------------------------------------------+
double CalcularG4_Agotamiento()
{
   int n = 6, m = n/2;
   double mp = 0, mu = 0, cp = 0, cu = 0;

   for(int i = 0; i < n && i < 100; i++)
   {
      double o = iOpen(_Symbol, PERIOD_M15, i);
      double c = iClose(_Symbol, PERIOD_M15, i);
      double h = iHigh(_Symbol, PERIOD_M15, i);
      double l = iLow(_Symbol, PERIOD_M15, i);
      if(h == 0 || l == 0) break;

      double r = h - l;
      if(r <= 0) continue;

      double me = r - MathAbs(c - o);
      double cu2 = MathAbs(c - o);

      if(i < m)
      {
         mu += me;
         cu += cu2;
      }
      else
      {
         mp += me;
         cp += cu2;
      }
   }

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return 0.0;

   double score_mechas = ((mu - mp) / atr14) * 50.0;
   double score_cuerpos = ((cp - cu) / atr14) * 50.0;

   return Clamp01100(Clamp01100(score_mechas) + Clamp01100(score_cuerpos));
}

//+------------------------------------------------------------------+
//| CALCULAR CALIDADES                                               |
//+------------------------------------------------------------------+
double CalcularCalidadSweep(double wick, double reclaim, double vol, int bars_ago, bool equal_hl)
{
   double t = Clamp01100((wick - 0.55) / 0.45 * 40) +
              Clamp01100((reclaim - 0.55) / 0.45 * 35) +
              Clamp01100((6 - bars_ago) / 5.0 * 15) +
              Clamp01100(MathMin(vol, 2) / 2 * 10);
   if(equal_hl) t = MathMin(100, t + 10);
   return Clamp01100(t);
}

double CalcularCalidadMSS(double wick, double reclaim, int mss_bars_ago)
{
   return Clamp01100(Clamp01100((wick - 0.55) / 0.45 * 40) +
                     Clamp01100((InpMSS_MaxAgeH4Bars - mss_bars_ago) /
                                (double)MathMax(InpMSS_MaxAgeH4Bars - 1, 1) * 30) +
                     Clamp01100((reclaim - 0.55) / 0.45 * 30));
}

double CalcularCalidadFVG(double fvg_size, double br_impulso, bool defendido)
{
   double t = Clamp01100((fvg_size - InpFVG_MinSizeATR) / (0.80 - InpFVG_MinSizeATR) * 45) +
              Clamp01100((br_impulso - InpFVG_BodyRatio) / (1.0 - InpFVG_BodyRatio) * 35);
   if(defendido) t = MathMin(100, t + 20);
   return Clamp01100(t);
}

double CalcularCalidadOB(double impulso, int ob_bars, double vol)
{
   return Clamp01100(Clamp01100((impulso - InpOB_ImpulseMin) / (2.5 - InpOB_ImpulseMin) * 50) +
                     Clamp01100((InpOB_Lookback - ob_bars) / (double)MathMax(InpOB_Lookback - 1, 1) * 30) +
                     Clamp01100(MathMin(vol, 2) / 2 * 20));
}

double CalcularSaludTendencial(int trend, double slope, string trend_d1, int dir)
{
   double p3 = 0;
   if((dir == 1 && trend_d1 == "ALCISTA") || (dir == -1 && trend_d1 == "BAJISTA"))
      p3 = 25;

   return Clamp01100(Clamp01100(MathMin(trend, 15) / 15.0 * 40) +
                     Clamp01100(MathMin(MathAbs(slope), 1) * 35) +
                     p3);
}

//+------------------------------------------------------------------+
//| CONFLUENCIAS                                                     |
//+------------------------------------------------------------------+
bool HuboSenalRecienteEnDireccion(string det, int dir, int n_velas)
{
   for(int i = 0; i < ArraySize(g_pending_signals); i++)
   {
      if(g_pending_signals[i].detector != det || g_pending_signals[i].direction != dir)
         continue;

      int a = g_pending_signals[i].entry_bar_shift;
      if(a >= 0 && a <= n_velas) return true;
   }
   return false;
}

double CalcularConfluenciaSweepFVG(int dir, bool fvg_ahora, double fvg_size)
{
   if(!HuboSenalRecienteEnDireccion("D2", dir, 6)) return 0;
   if(!fvg_ahora) return 40;
   return Clamp01100(60 + Clamp01100((fvg_size - InpFVG_MinSizeATR) / 0.60 * 40) * 0.4);
}

double CalcularConfluenciaCompleta(int dir, bool fvg_ahora, double fvg_size)
{
   int p = 0;
   if(HuboSenalRecienteEnDireccion("D5", dir, 8)) p++;
   if(HuboSenalRecienteEnDireccion("D2", dir, 8)) p++;
   if(fvg_ahora) p++;

   if(p == 0) return 0;
   if(p == 1) return 25;
   if(p == 2) return 60;
   return Clamp01100(85 + Clamp01100((fvg_size - InpFVG_MinSizeATR) / 0.60 * 15) * 0.15);
}

//+------------------------------------------------------------------+
//| CALCULAR VENCIMIENTO                                             |
//+------------------------------------------------------------------+
int CalcularVencimiento(const Signal &sig)
{
   double atr = sig.atr14;
   string detector = sig.detector;

   if(detector == "D1")
   {
      if(atr > 20) return 2;
      return 1;
   }
   else if(detector == "D2" || detector == "D2_ANTICIPACION")
   {
      if(sig.kill_zone != "NONE") return 1;
      if(atr > 15) return 1;
      return 2;
   }
   else if(detector == "D3" || detector == "D3_DEF")
   {
      if(sig.detector == "D3_DEF") return 2;
      if(atr > 20) return 2;
      return 1;
   }
   else if(detector == "D4")
   {
      if(sig.ob_confluence) return 1;
      return 2;
   }
   else if(detector == "D5")
   {
      if(sig.kill_zone != "NONE") return 2;
      return 4;
   }

   return 2;
}

//+------------------------------------------------------------------+
//| GENERARHIPOTESIS — CORREGIDO (FIX objetivo)                     |
//+------------------------------------------------------------------+
void GenerarHipotesis(Signal &sig)
{
   sig.hipotesis_expiry_velas = CalcularVencimiento(sig);
   sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15;

   string zona;
   if(EsZonaPremiumDiscount(sig.entry_price, zona))
   {
      sig.hipotesis_zona = zona;
   }

   // --- CALCULAR OBJETIVO NUMÉRICO (FIX) ---
   if(g_estructura.sweep_nivel > 0) {
      sig.hipotesis_objetivo = g_estructura.sweep_nivel;
   } else {
      double atr = sig.atr14 * _Point;
      if(atr <= 0) atr = g_atr14_buffer[0];
      sig.hipotesis_objetivo = (sig.direction == 1) ?
          sig.entry_price + atr * 1.5 :
          sig.entry_price - atr * 1.5;
   }

   string causa = "", efecto = "", razon = "", invalidez = "";
   int prob_base = 55;
   double atr14 = sig.atr14 * _Point;
   if(atr14 <= 0) atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   if(sig.detector == "D1")
   {
      string dir_ruptura = (sig.direction == +1) ? "alcista" : "bajista";
      string nivel = DoubleToString(sig.nivel_estructural, _Digits);
      string estructura = sig.estructura_direccion;
      string invalidez_nivel = (sig.direction == +1) ?
         DoubleToString(sig.nivel_estructural - atr14 * 0.3, _Digits) :
         DoubleToString(sig.nivel_estructural + atr14 * 0.3, _Digits);

      causa = "Ruptura " + dir_ruptura + " de " + nivel;
      efecto = "va a provocar continuación " + dir_ruptura + " hacia " + DoubleToString(sig.hipotesis_objetivo, _Digits);
      razon = "porque la vela actual rompe con fuerza (BR=" +
              DoubleToString(sig.br, 2) + ") y la tendencia " + estructura + " confirma";
      invalidez = "Si rompe " + invalidez_nivel + " en contra, se invalida";

      prob_base = 60;
      if(sig.br > 0.70) prob_base += 5;
      if(sig.bs > 1.0) prob_base += 5;
      if(sig.g1_compresion >= 60) prob_base += 5;
      if(sig.g2_persistencia >= 60) prob_base += 5;
      if(sig.kill_zone != "NONE") prob_base += 5;
   }
   else if(sig.detector == "D2" || sig.detector == "D2_ANTICIPACION")
   {
      string zona_text = sig.hipotesis_zona;
      string accion = (sig.direction == +1) ? "rebote alcista" : "rechazo bajista";
      string nivel = DoubleToString(sig.level_swept, _Digits);
      string estructura = sig.estructura_direccion;
      string invalidez_nivel = (sig.direction == +1) ?
         DoubleToString(sig.level_swept - atr14 * 0.3, _Digits) :
         DoubleToString(sig.level_swept + atr14 * 0.3, _Digits);

      causa = "Sweep en " + nivel + " en zona " + zona_text;
      efecto = "va a provocar " + accion + " hacia " + DoubleToString(sig.hipotesis_objetivo, _Digits);
      razon = "porque el sweep liquida stops y la tendencia " + estructura + " confirma";
      invalidez = "Si rompe " + invalidez_nivel + ", se invalida";

      prob_base = 65;
      if(sig.equal_hl_detected) prob_base += 5;
      if(sig.hipotesis_zona == "PREMIUM" && sig.direction == -1) prob_base += 5;
      if(sig.hipotesis_zona == "DISCOUNT" && sig.direction == +1) prob_base += 5;
      if(sig.kill_zone != "NONE") prob_base += 5;
      if(sig.sweep_volume_ratio > 1.8) prob_base += 5;
      if(sig.g4_agotamiento >= 65) prob_base -= 10;
   }
   else if(sig.detector == "D3" || sig.detector == "D3_DEF")
   {
      string dir_fvg = (sig.direction == -1) ? "BAJISTA" : "ALCISTA";
      string zona_text = sig.hipotesis_zona;
      string accion = (sig.direction == -1) ? "rechazo bajista" : "rebote alcista";
      string defensa = (sig.detector == "D3_DEF") ?
         "Los " + (sig.direction == -1 ? "vendedores" : "compradores") + " defienden la zona" :
         "La zona está activa";
      string estructura = sig.estructura_direccion;
      string invalidez_nivel = (sig.direction == -1) ?
         DoubleToString(sig.fvg_top, _Digits) :
         DoubleToString(sig.fvg_bottom, _Digits);
      string direccion_invalidez = (sig.direction == -1) ? "al alza" : "a la baja";

      causa = "FVG " + dir_fvg + " en zona " + zona_text;
      efecto = "va a provocar " + accion + " hacia " + DoubleToString(sig.hipotesis_objetivo, _Digits);
      razon = "porque " + defensa + " y la tendencia " + estructura + " confirma";
      invalidez = "Si rompe " + invalidez_nivel + " " + direccion_invalidez + ", se invalida";

      prob_base = 60;
      if(sig.detector == "D3_DEF") prob_base += 5;
      if(sig.hipotesis_zona == "PREMIUM" && sig.direction == -1) prob_base += 5;
      if(sig.hipotesis_zona == "DISCOUNT" && sig.direction == +1) prob_base += 5;
      if(sig.kill_zone != "NONE") prob_base += 5;
      if(sig.g1_compresion >= 60) prob_base += 5;
      if(sig.mss_aligned) prob_base += 5;
      if(sig.g4_agotamiento >= 65) prob_base -= 10;
   }
   else if(sig.detector == "D4")
   {
      string accion = (sig.direction == +1) ? "rebote alcista" : "rechazo bajista";
      string nivel = DoubleToString((sig.ob_high + sig.ob_low) / 2.0, _Digits);
      string estructura = sig.estructura_direccion;
      string invalidez_nivel = (sig.direction == +1) ?
         DoubleToString(sig.ob_low, _Digits) :
         DoubleToString(sig.ob_high, _Digits);

      causa = "Order Block en " + nivel;
      efecto = "va a provocar " + accion + " hacia " + DoubleToString(sig.hipotesis_objetivo, _Digits);
      razon = "porque el OB representa acumulación/distribución y la tendencia " + estructura + " confirma";
      invalidez = "Si rompe " + invalidez_nivel + ", se invalida";

      prob_base = 60;
      if(sig.ob_impulse_atr > 1.5) prob_base += 5;
      if(sig.ob_bars_ago <= 3) prob_base += 5;
      if(sig.kill_zone != "NONE") prob_base += 5;
      if(sig.g1_compresion >= 60) prob_base += 5;
   }
   else if(sig.detector == "D5")
   {
      string dir_mss = sig.mss_direction;
      string accion = (sig.direction == +1) ? "continuación alcista" : "continuación bajista";
      string nivel = DoubleToString(sig.level_swept, _Digits);
      string estructura = sig.estructura_direccion;
      string invalidez_nivel = (sig.direction == +1) ?
         DoubleToString(sig.level_swept - atr14 * 0.5, _Digits) :
         DoubleToString(sig.level_swept + atr14 * 0.5, _Digits);

      causa = "MSS H4 " + dir_mss + " con sweep en " + nivel;
      efecto = "va a provocar " + accion + " hacia " + DoubleToString(sig.hipotesis_objetivo, _Digits);
      razon = "porque el cambio de estructura en H4 confirma la dirección y el sweep valida la entrada";
      invalidez = "Si rompe " + invalidez_nivel + ", se invalida";

      prob_base = 70;
      if(sig.mss_bars_ago_h4 <= 4) prob_base += 5;
      if(sig.kill_zone != "NONE") prob_base += 5;
      if(sig.g1_compresion >= 60) prob_base += 5;
      if(sig.g2_persistencia >= 60) prob_base += 5;
   }

   prob_base = MathMin(95, MathMax(30, prob_base));

   sig.hipotesis_prob_min = prob_base - 5;
   sig.hipotesis_prob_max = prob_base + 5;
   sig.hipotesis_prob_min = MathMax(30, sig.hipotesis_prob_min);
   sig.hipotesis_prob_max = MathMin(95, sig.hipotesis_prob_max);

   sig.hipotesis_causa = causa;
   sig.hipotesis_efecto = efecto;
   sig.hipotesis_razon = razon;
   sig.hipotesis_invalidez = invalidez;
   sig.hipotesis_texto = causa + "\n" + efecto + "\n" + razon + "\n" + invalidez;
}

//+------------------------------------------------------------------+
//| DETECTMSS_H4                                                     |
//+------------------------------------------------------------------+
bool DetectMSS_H4(int &bars_ago, string &dir, double &level)
{
   datetime currentH4Bar = iTime(_Symbol, PERIOD_H4, 0);

   if(g_mss_cache.valid && g_mss_cache.calc_time == currentH4Bar)
   {
      bars_ago = g_mss_cache.bars_ago;
      dir = g_mss_cache.dir;
      level = g_mss_cache.level;
      return true;
   }

   for(int i = 1; i <= InpMSS_LookbackH4 && i < 50; i++)
   {
      double close_i = iClose(_Symbol, PERIOD_H4, i);
      if(close_i == 0) continue;

      double prior_high = iHigh(_Symbol, PERIOD_H4, i + 1);
      double prior_low = iLow(_Symbol, PERIOD_H4, i + 1);

      for(int k = i + 1; k <= i + InpMSS_LookbackH4 && k < 50; k++)
      {
         double hk = iHigh(_Symbol, PERIOD_H4, k);
         double lk = iLow(_Symbol, PERIOD_H4, k);
         if(hk == 0 || lk == 0) break;
         if(hk > prior_high) prior_high = hk;
         if(lk < prior_low) prior_low = lk;
      }

      if(close_i > prior_high)
      {
         bars_ago = i;
         dir = "ALCISTA";
         level = prior_high;

         g_mss_cache.valid = true;
         g_mss_cache.calc_time = currentH4Bar;
         g_mss_cache.bars_ago = i;
         g_mss_cache.dir = dir;
         g_mss_cache.level = level;
         return true;
      }

      if(close_i < prior_low)
      {
         bars_ago = i;
         dir = "BAJISTA";
         level = prior_low;

         g_mss_cache.valid = true;
         g_mss_cache.calc_time = currentH4Bar;
         g_mss_cache.bars_ago = i;
         g_mss_cache.dir = dir;
         g_mss_cache.level = level;
         return true;
      }
   }

   g_mss_cache.valid = false;
   return false;
}

//+------------------------------------------------------------------+
//| PROCESSINTRABAR                                                  |
//+------------------------------------------------------------------+
void ProcessIntraBar()
{
   if(!UpdateIndicators()) return;

   MeasureReturns();
   UpdateATRHistory();

   int max_lookback = MathMax(InpN_Ruptura, MathMax(InpSweep_N, MathMax(InpOB_Lookback, 10)));
   double vol_ratio = GetVolumeRatioCached(0, max_lookback);

   datetime currentBar = iTime(_Symbol, PERIOD_M15, 0);
   string session = GetSession(currentBar);
   string kill_zone = GetKillZone(currentBar);
   bool vol_exp = IsVolatilityExpanding();
   bool vol_comp = IsVolatilityCompressing();
   string trend_d1 = GetTrendD1(g_ema50_d1_buffer, g_ema200_d1_buffer);

   if(currentBar != g_lastG_calcBar)
   {
      g_g1_compresion = CalcularG1_Compresion();
      g_g2_persistencia = CalcularG2_Persistencia();
      g_g3_eficiencia = CalcularG3_Eficiencia();
      g_g4_agotamiento = CalcularG4_Agotamiento();
      g_lastG_calcBar = currentBar;
   }

   datetime currentH1Bar = iTime(_Symbol, PERIOD_H1, 0);
   if(currentH1Bar != g_lastStructUpdate || g_estructura.timestamp == 0)
   {
      ActualizarEstructura();
      g_lastStructUpdate = currentH1Bar;
   }

   datetime currentH4Bar = iTime(_Symbol, PERIOD_H4, 0);
   if(g_mss_cache.calc_time != currentH4Bar)
      g_mss_cache.valid = false;

   g_zona_cache.valid = false;

   ArrayResize(g_candidatas_vela, 0);

   MotorD1_IntraBar(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);
   MotorD2_LiquiditySweep(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);

   if(InpD2_Anticipar)
      MotorD2_Anticipacion(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);

   MotorD3_IntraBar(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);
   MotorD4_OrderBlockConfluence(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);
   MotorD5_MSS_Sweep(vol_ratio, session, kill_zone, vol_exp, vol_comp, trend_d1);

   ResolverConfluenciasYRutear();
}

//+------------------------------------------------------------------+
//| RESOLVERCONFLUENCIASYRUTEAR                                      |
//+------------------------------------------------------------------+
void ResolverConfluenciasYRutear()
{
   int total = ArraySize(g_candidatas_vela);
   if(total == 0) return;

   for(int i = 0; i < total; i++)
   {
      Signal sig = g_candidatas_vela[i];

      sig.hipotesis_expiry_velas = CalcularVencimiento(sig);
      sig.hipotesis_expiry_minutos = sig.hipotesis_expiry_velas * 15;

      string zona;
      if(EsZonaPremiumDiscount(sig.entry_price, zona))
         sig.hipotesis_zona = zona;

      GenerarHipotesis(sig);
      RouteSignal(sig);
   }
}

//+------------------------------------------------------------------+
//| ROUTESIGNAL                                                      |
//+------------------------------------------------------------------+
void RouteSignal(Signal &sig)
{
   sig.calidad_sweep = Clamp01100(sig.calidad_sweep);
   sig.calidad_mss = Clamp01100(sig.calidad_mss);
   sig.calidad_fvg = Clamp01100(sig.calidad_fvg);
   sig.calidad_ob = Clamp01100(sig.calidad_ob);
   sig.salud_tendencial = Clamp01100(sig.salud_tendencial);
   sig.contexto_estructural = Clamp01100(sig.contexto_estructural);

   int size = ArraySize(g_pending_signals);
   if(size >= MAX_PENDING_SIGNALS)
   {
      for(int i = 0; i < size - 1; i++)
         g_pending_signals[i] = g_pending_signals[i + 1];
      ArrayResize(g_pending_signals, size - 1);
      size--;
   }

   ArrayResize(g_pending_signals, size + 1);
   g_pending_signals[size] = sig;
   WriteSignalToCSV(sig);

   bool route = false;
   if(sig.detector == "D1" && InpColaD1_Enabled) route = true;
   if(sig.detector == "D2" && InpColaD2_Enabled) route = true;
   if(sig.detector == "D2_ANTICIPACION" && InpColaD2_Enabled) route = true;
   if((sig.detector == "D3" || sig.detector == "D3_DEF") && InpColaD3_Enabled) route = true;
   if(sig.detector == "D4" && InpColaD4_Enabled) route = true;
   if(sig.detector == "D5" && InpColaD5_Enabled) route = true;

   if(route)
   {
      if(!AcquireLock()) return;

      int h = FileOpen(InpColaSenalesFile, FILE_READ | FILE_WRITE | FILE_CSV | FILE_COMMON | FILE_ANSI);
      if(h != INVALID_HANDLE)
      {
         FileSeek(h, 0, SEEK_END);
         FileWrite(h,
            TimeToString(sig.entry_time, TIME_DATE | TIME_SECONDS) + "," +
            sig.symbol + "," + IntegerToString(sig.direction) + "," +
            DoubleToString(sig.entry_price, _Digits) + "," + sig.detector + "," +
            sig.tipo + "," + sig.session + "," + sig.kill_zone + "," +
            DoubleToString(sig.contexto_estructural, 1) + "," +
            sig.estructura_direccion);
         FileClose(h);
      }
      ReleaseLock();
   }

   string msg;
   BuildAlertText(sig, msg);

   // FIX: Cooldown unificado a 5 segundos
   if(TimeCurrent() - g_lastNtfyTime >= 5)
   {
      if(SendNtfyMessage(msg))
      {
         g_lastAlertTime = TimeCurrent();
         g_lastNtfyTime = TimeCurrent();
         Print("ALERTA: ", sig.detector, " ", sig.symbol, " dir=", sig.direction);
      }
      else
      {
         QueueAlert(msg);
         Print("Alerta encolada: ", sig.detector);
      }
   }
   else
   {
      QueueAlert(msg);
      Print("Alerta encolada (cooldown): ", sig.detector);
   }

   Print("SEÑAL ", sig.detector, " [", sig.tipo, "] ", sig.symbol,
         " dir=", sig.direction, " precio=", DoubleToString(sig.entry_price, _Digits),
         " Prob=", IntegerToString(sig.hipotesis_prob_min), "-",
         IntegerToString(sig.hipotesis_prob_max), "%");
}

//+------------------------------------------------------------------+
//| MOTOR D1 - RUPTURA DE RANGO (INTRAVELA)                         |
//+------------------------------------------------------------------+
void MotorD1_IntraBar(double vol_ratio, string session, string kill_zone,
                      bool vol_exp, bool vol_comp, string trend_d1)
{
   double high0 = iHigh(_Symbol, PERIOD_M15, 0);
   double low0 = iLow(_Symbol, PERIOD_M15, 0);
   double close0 = iClose(_Symbol, PERIOD_M15, 0);
   double open0 = iOpen(_Symbol, PERIOD_M15, 0);

   if(high0 == 0 || low0 == 0 || close0 == 0) return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   double highest_high = iHigh(_Symbol, PERIOD_M15, 1);
   double lowest_low = iLow(_Symbol, PERIOD_M15, 1);

   for(int k = 2; k <= InpN_Ruptura + 1 && k < 100; k++)
   {
      double h = iHigh(_Symbol, PERIOD_M15, k);
      double l = iLow(_Symbol, PERIOD_M15, k);
      if(h == 0 || l == 0) break;
      if(h > highest_high) highest_high = h;
      if(l < lowest_low) lowest_low = l;
   }

   int direction = 0;
   double nivel_ruptura = 0;
   double penetracion = 0;

   if(high0 > highest_high)
   {
      direction = +1;
      nivel_ruptura = highest_high;
      penetracion = (high0 - highest_high) / atr14;
   }
   else if(low0 < lowest_low)
   {
      direction = -1;
      nivel_ruptura = lowest_low;
      penetracion = (lowest_low - low0) / atr14;
   }

   if(direction == 0) return;
   if(penetracion < InpD1_ATRThreshold) return;

   double rango0 = high0 - low0;
   if(rango0 <= 0) return;

   double br0 = MathAbs(close0 - open0) / rango0;
   if(br0 < InpBodyRatio_Min) return;

   if(InpD1_UseVolume)
   {
      double vol_ratio_signal = GetVolumeRatioCached(0, 20);
      if(vol_ratio_signal < InpD1_MinVolume) return;
   }

   if(InpD1_UseRetest)
   {
      bool retested = false;

      if(direction == +1)
      {
         // FIX: Solo retest real — precio debe tocar o cruzar el nivel y cerrar al otro lado
         if(low0 <= nivel_ruptura && close0 > nivel_ruptura)
            retested = true;
      }
      else
      {
         if(high0 >= nivel_ruptura && close0 < nivel_ruptura)
            retested = true;
      }

      if(!retested) return;
   }

   ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 0), "D1", direction, nivel_ruptura);
   if(IsDuplicateSignal(id) || HasDetectorFiredThisBar("D1", direction, nivel_ruptura))
      return;

   MarkDetectorFired("D1", direction, nivel_ruptura);

   Signal sig;
   sig.id = id;
   sig.entry_time = iTime(_Symbol, PERIOD_M15, 0);
   sig.entry_bar_shift = 0;
   sig.symbol = _Symbol;
   sig.direction = direction;
   sig.entry_price = close0;
   sig.detector = "D1";
   sig.es_intravela = true;

   sig.br = br0;
   sig.bs = penetracion;
   sig.nivel_estructural = nivel_ruptura;
   sig.atr14 = atr14 / _Point;
   sig.session = session;
   sig.kill_zone = kill_zone;
   sig.estructura_direccion = g_estructura.dir_estructura;
   sig.g1_compresion = g_g1_compresion;
   sig.g2_persistencia = g_g2_persistencia;
   sig.g4_agotamiento = g_g4_agotamiento;
   sig.tipo = ClasificarD1(br0, penetracion, session);

   sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(),
      (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14, trend_d1, direction);

   double dist;
   sig.contexto_estructural = EvaluarContextoEstructural(direction, nivel_ruptura, "D1", trend_d1, dist);
   sig.distancia_al_sweep = dist;
   sig.en_zona_estructural = g_estructura.en_zona;

   AgregarCandidata(sig);
}

//+------------------------------------------------------------------+
//| MOTOR D2 - LIQUIDITY SWEEP + RECLAIM (INTRAVELA)                |
//+------------------------------------------------------------------+
void MotorD2_LiquiditySweep(double vol_ratio, string session, string kill_zone,
                            bool vol_exp, bool vol_comp, string trend_d1)
{
   double close0 = iClose(_Symbol, PERIOD_M15, 0);
   double open0 = iOpen(_Symbol, PERIOD_M15, 0);
   double high0 = iHigh(_Symbol, PERIOD_M15, 0);
   double low0 = iLow(_Symbol, PERIOD_M15, 0);

   if(close0 == 0 || high0 == 0 || low0 == 0) return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   int sweep_bar = -1, sweep_dir = 0;
   double wick_found = 0, vol_found = 0, level = 0;
   bool equal_hl = false;

   for(int i = 1; i <= 2 && i < 100; i++)
   {
      double hi = iHigh(_Symbol, PERIOD_M15, i);
      double li = iLow(_Symbol, PERIOD_M15, i);
      if(hi == 0 || li == 0) continue;

      double oi = iOpen(_Symbol, PERIOD_M15, i);
      double ci = iClose(_Symbol, PERIOD_M15, i);
      double ri = hi - li;
      if(ri <= 0) continue;

      double ph = iHigh(_Symbol, PERIOD_M15, i + 1);
      double pl = iLow(_Symbol, PERIOD_M15, i + 1);

      for(int k = i + 1; k <= i + InpSweep_N && k < 100; k++)
      {
         double hk = iHigh(_Symbol, PERIOD_M15, k);
         double lk = iLow(_Symbol, PERIOD_M15, k);
         if(hk == 0 || lk == 0) break;
         if(hk > ph) ph = hk;
         if(lk < pl) pl = lk;
      }

      if(ph == 0 || pl == 0) continue;

      bool per_high = (hi > ph) && (ci < ph);
      bool per_low = (li < pl) && (ci > pl);

      if(!per_high && !per_low) continue;

      double wr;
      int dc;
      double lc;

      if(per_high)
      {
         wr = (hi - MathMax(oi, ci)) / ri;
         dc = -1;
         lc = ph;
      }
      else
      {
         wr = (MathMin(oi, ci) - li) / ri;
         dc = +1;
         lc = pl;
      }

      if(wr < InpSweepWickMin) continue;

      bool eq = false;
      for(int j = i + 1; j <= i + InpEqualHL_Window && j < 100; j++)
      {
         double hj = iHigh(_Symbol, PERIOD_M15, j);
         double lj = iLow(_Symbol, PERIOD_M15, j);
         if(hj == 0 || lj == 0) break;

         if(per_high)
         {
            if(MathAbs(hj - lc) <= InpEqualHL_Tol * atr14)
            {
               eq = true;
               break;
            }
         }
         else
         {
            if(MathAbs(lj - lc) <= InpEqualHL_Tol * atr14)
            {
               eq = true;
               break;
            }
         }
      }

      if(eq) equal_hl = true;

      sweep_bar = i;
      sweep_dir = dc;
      wick_found = wr;
      vol_found = GetVolumeRatio(i, InpSweep_N);
      level = lc;
      break;
   }

   if(sweep_bar == -1 || sweep_bar > 2 || MathAbs(close0 - level) > atr14 * 2.0)
      return;

   double br_reclaim = (high0 - low0 > 0) ? MathAbs(close0 - open0) / (high0 - low0) : 0;

   bool reclaim_ok = (sweep_dir == +1 && close0 > open0 && close0 > level) ||
                     (sweep_dir == -1 && close0 < open0 && close0 < level);

   if(!reclaim_ok || br_reclaim < InpReclaimBodyMin) return;

   ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 0), "D2", sweep_dir, level);
   if(IsDuplicateSignal(id) || HasDetectorFiredThisBar("D2", sweep_dir, level)) return;

   MarkDetectorFired("D2", sweep_dir, level);

   Signal sig;
   sig.id = id;
   sig.entry_time = iTime(_Symbol, PERIOD_M15, 0);
   sig.entry_bar_shift = 0;
   sig.symbol = _Symbol;
   sig.direction = sweep_dir;
   sig.entry_price = close0;
   sig.detector = "D2";
   sig.es_intravela = true;
   sig.level_swept = level;
   sig.sweep_wick_ratio = wick_found;
   sig.sweep_volume_ratio = vol_found;
   sig.reclaim_body_ratio = br_reclaim;
   sig.equal_hl_detected = equal_hl;
   sig.atr14 = atr14 / _Point;
   sig.session = session;
   sig.kill_zone = kill_zone;
   sig.estructura_direccion = g_estructura.dir_estructura;
   sig.g1_compresion = g_g1_compresion;
   sig.g2_persistencia = g_g2_persistencia;
   sig.g4_agotamiento = g_g4_agotamiento;
   sig.tipo = ClasificarD2(wick_found, vol_found, br_reclaim, equal_hl);

   sig.calidad_sweep = CalcularCalidadSweep(wick_found, br_reclaim, vol_found, sweep_bar, equal_hl);
   sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(),
      (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14, trend_d1, sweep_dir);

   double dist;
   sig.contexto_estructural = EvaluarContextoEstructural(sweep_dir, level, "D2", trend_d1, dist);
   sig.distancia_al_sweep = dist;
   sig.en_zona_estructural = g_estructura.en_zona;

   AgregarCandidata(sig);
}

//+------------------------------------------------------------------+
//| MOTOR D2_ANTICIPACION - ALERTA TEMPRANA (INTRAVELA)             |
//+------------------------------------------------------------------+
void MotorD2_Anticipacion(double vol_ratio, string session, string kill_zone,
                          bool vol_exp, bool vol_comp, string trend_d1)
{
   double high0 = iHigh(_Symbol, PERIOD_M15, 0);
   double low0 = iLow(_Symbol, PERIOD_M15, 0);
   double close0 = iClose(_Symbol, PERIOD_M15, 0);
   double open0 = iOpen(_Symbol, PERIOD_M15, 0);

   if(high0 == 0 || low0 == 0 || close0 == 0 || open0 == 0) return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   double prior_high = iHigh(_Symbol, PERIOD_M15, 1);
   double prior_low = iLow(_Symbol, PERIOD_M15, 1);

   for(int k = 2; k <= InpSweep_N && k < 100; k++)
   {
      double h = iHigh(_Symbol, PERIOD_M15, k);
      double l = iLow(_Symbol, PERIOD_M15, k);
      if(h == 0 || l == 0) break;
      if(h > prior_high) prior_high = h;
      if(l < prior_low) prior_low = l;
   }

   bool sweep_high = (high0 > prior_high);
   bool sweep_low = (low0 < prior_low);

   if(!sweep_high && !sweep_low) return;

   int sweep_dir = sweep_high ? -1 : +1;
   double nivel_barrido = sweep_high ? prior_high : prior_low;

   double range = high0 - low0;
   if(range <= 0) return;

   double wick_ratio = sweep_high ? (high0 - MathMax(open0, close0)) / range :
                                    (MathMin(open0, close0) - low0) / range;

   if(wick_ratio < InpSweepWickMin * 0.6) return;

   int confluencias = 0;

   bool hay_fvg = false;
   for(int i = 2; i <= 5 && i < 100; i++)
   {
      double ha = iHigh(_Symbol, PERIOD_M15, i);
      double la = iLow(_Symbol, PERIOD_M15, i);
      double hb = iHigh(_Symbol, PERIOD_M15, i - 1);
      double lb = iLow(_Symbol, PERIOD_M15, i - 1);
      double hc = iHigh(_Symbol, PERIOD_M15, i - 2);
      double lc2 = iLow(_Symbol, PERIOD_M15, i - 2);

      if(ha == 0 || la == 0 || hb == 0 || lb == 0 || hc == 0 || lc2 == 0)
         continue;

      if(ha < lc2)
      {
         double ce = lc2 - (lc2 - ha) * 0.5;
         if(MathAbs(nivel_barrido - ce) < atr14 * 0.5)
         {
            hay_fvg = true;
            break;
         }
      }
      else if(la > hc)
      {
         double ce = la - (la - hc) * 0.5;
         if(MathAbs(nivel_barrido - ce) < atr14 * 0.5)
         {
            hay_fvg = true;
            break;
         }
      }
   }
   if(hay_fvg) confluencias++;

   bool hay_ob = false;
   for(int i = 2; i <= 4 && i < 100; i++)
   {
      double oi = iOpen(_Symbol, PERIOD_M15, i);
      double ci = iClose(_Symbol, PERIOD_M15, i);
      double hi = iHigh(_Symbol, PERIOD_M15, i);
      double li = iLow(_Symbol, PERIOD_M15, i);
      double ri = hi - li;
      if(ri <= 0 || hi == 0 || li == 0) continue;

      if(MathAbs(ci - oi) / ri < InpOB_BodyMin) continue;

      double nc = iClose(_Symbol, PERIOD_M15, i - 1);
      double imp = MathAbs(nc - ci) / atr14;
      if(imp < InpOB_ImpulseMin) continue;

      if(MathAbs(nivel_barrido - (hi + li) / 2.0) < atr14 * 0.5)
      {
         hay_ob = true;
         break;
      }
   }
   if(hay_ob) confluencias++;

   bool hay_mss = false;
   int mss_bars;
   string mss_dir;
   double mss_level;

   if(DetectMSS_H4(mss_bars, mss_dir, mss_level))
   {
      int md = (mss_dir == "ALCISTA") ? +1 : -1;
      if(md == sweep_dir)
      {
         hay_mss = true;
         confluencias++;
      }
   }

   if(confluencias >= 2)
   {
      ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 0), "D2_ANTICIPACION", sweep_dir, nivel_barrido);
      if(IsDuplicateSignal(id) || HasDetectorFiredThisBar("D2_ANTICIPACION", sweep_dir, nivel_barrido))
         return;

      MarkDetectorFired("D2_ANTICIPACION", sweep_dir, nivel_barrido);

      Signal sig;
      sig.id = id;
      sig.entry_time = iTime(_Symbol, PERIOD_M15, 0);
      sig.entry_bar_shift = 0;
      sig.symbol = _Symbol;
      sig.direction = sweep_dir;
      sig.entry_price = close0;
      sig.detector = "D2_ANTICIPACION";
      sig.es_intravela = true;
      sig.level_swept = nivel_barrido;
      sig.sweep_wick_ratio = wick_ratio;
      sig.sweep_volume_ratio = vol_ratio;
      sig.atr14 = atr14 / _Point;
      sig.session = session;
      sig.kill_zone = kill_zone;
      sig.estructura_direccion = g_estructura.dir_estructura;
      sig.g1_compresion = g_g1_compresion;
      sig.g2_persistencia = g_g2_persistencia;
      sig.g4_agotamiento = g_g4_agotamiento;
      sig.tipo = ClasificarD2_Anticipacion(wick_ratio, vol_ratio, confluencias);

      sig.calidad_sweep = CalcularCalidadSweep(wick_ratio, 0, vol_ratio, 0, false);
      sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(),
         (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14, trend_d1, sweep_dir);

      double dist;
      sig.contexto_estructural = EvaluarContextoEstructural(sweep_dir, nivel_barrido, "D2_ANTICIPACION", trend_d1, dist);
      sig.distancia_al_sweep = dist;
      sig.en_zona_estructural = g_estructura.en_zona;

      sig.conf_sweep_fvg = hay_fvg ? CalcularConfluenciaSweepFVG(sweep_dir, true, 0) : 0;
      sig.conf_completa = CalcularConfluenciaCompleta(sweep_dir, hay_fvg, 0);

      AgregarCandidata(sig);
   }
}

//+------------------------------------------------------------------+
//| MOTOR D3 - FAIR VALUE GAP (INTRAVELA)                           |
//+------------------------------------------------------------------+
void MotorD3_IntraBar(double vol_ratio, string session, string kill_zone,
                      bool vol_exp, bool vol_comp, string trend_d1)
{
   double ha = iHigh(_Symbol, PERIOD_M15, 2);
   double la = iLow(_Symbol, PERIOD_M15, 2);
   double hb = iHigh(_Symbol, PERIOD_M15, 1);
   double lb = iLow(_Symbol, PERIOD_M15, 1);
   double cb = iClose(_Symbol, PERIOD_M15, 1);
   double ob = iOpen(_Symbol, PERIOD_M15, 1);
   double hc = iHigh(_Symbol, PERIOD_M15, 0);
   double lc2 = iLow(_Symbol, PERIOD_M15, 0);

   if(ha == 0 || la == 0 || hb == 0 || lb == 0 || hc == 0 || lc2 == 0)
      return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   bool fvg_alcista = (ha < lc2);
   bool fvg_bajista = (la > hc);

   if(!fvg_alcista && !fvg_bajista) return;

   double fvg_size = 0, fvg_top = 0, fvg_bottom = 0;
   int direction = 0;

   if(fvg_alcista)
   {
      fvg_size = lc2 - ha;
      fvg_top = lc2;
      fvg_bottom = ha;
      direction = +1;
   }
   else
   {
      fvg_size = la - hc;
      fvg_top = la;
      fvg_bottom = hc;
      direction = -1;
   }

   if(fvg_size <= 0) return;

   double fvg_size_atr = fvg_size / atr14;
   double br_b = (hb - lb > 0) ? MathAbs(cb - ob) / (hb - lb) : 0;
   bool dir_ok = (fvg_alcista && cb > ob) || (fvg_bajista && cb < ob);

   if(fvg_size_atr < InpFVG_MinSizeATR || br_b < InpFVG_BodyRatio || !dir_ok)
      return;

   double mit_level = fvg_bottom + (fvg_top - fvg_bottom) * InpFVG_MitigUmbral;
   double price0 = iClose(_Symbol, PERIOD_M15, 0);

   bool mitigado = (direction == +1 && price0 <= mit_level) ||
                   (direction == -1 && price0 >= mit_level);

   // FIX: Entrada dentro del FVG también es válida (precio en zona de valor)
   bool defendido = false;
   bool dentro_fvg = false;

   if(direction == +1 && price0 > fvg_top)
      defendido = true;
   if(direction == -1 && price0 < fvg_bottom)
      defendido = true;
   if(direction == +1 && price0 >= fvg_bottom && price0 <= fvg_top)
      dentro_fvg = true;
   if(direction == -1 && price0 <= fvg_top && price0 >= fvg_bottom)
      dentro_fvg = true;

   // FIX: D3_DEF si defendido o dentro del FVG; D3 solo si mitigado pero no defendido
   string det = (defendido || dentro_fvg) ? "D3_DEF" : "D3";

   ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 1), det, direction, fvg_top);
   if(IsDuplicateSignal(id) || HasDetectorFiredThisBar(det, direction, fvg_top))
      return;

   MarkDetectorFired(det, direction, fvg_top);

   Signal sig;
   sig.id = id;
   sig.entry_time = iTime(_Symbol, PERIOD_M15, 1);
   sig.entry_bar_shift = 1;
   sig.symbol = _Symbol;
   sig.direction = direction;
   sig.entry_price = price0;
   sig.detector = det;
   sig.es_intravela = true;
   sig.fvg_top = fvg_top;
   sig.fvg_bottom = fvg_bottom;
   sig.fvg_size_atr = fvg_size_atr;
   sig.fvg_mitigated = mitigado;
   sig.atr14 = atr14 / _Point;
   sig.session = session;
   sig.kill_zone = kill_zone;
   sig.estructura_direccion = g_estructura.dir_estructura;

   int mss_bars;
   string mss_dir;
   double mss_level;
   sig.mss_aligned = DetectMSS_H4(mss_bars, mss_dir, mss_level);
   sig.mss_bars_ago_h4 = mss_bars;
   sig.mss_direction = mss_dir;
   sig.mss_level = mss_level;

   sig.g1_compresion = g_g1_compresion;
   sig.g2_persistencia = g_g2_persistencia;
   sig.g4_agotamiento = g_g4_agotamiento;

   double slope = (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14;
   sig.tipo = ClasificarD3(fvg_size_atr, br_b, GetTrendVelas(), slope);

   sig.calidad_fvg = CalcularCalidadFVG(fvg_size_atr, br_b, defendido);
   sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(), slope, trend_d1, direction);

   double dist;
   sig.contexto_estructural = EvaluarContextoEstructural(direction, fvg_top, det, trend_d1, dist);
   sig.distancia_al_sweep = dist;
   sig.en_zona_estructural = g_estructura.en_zona;

   sig.conf_sweep_fvg = CalcularConfluenciaSweepFVG(direction, true, fvg_size_atr);
   sig.conf_completa = CalcularConfluenciaCompleta(direction, true, fvg_size_atr);

   AgregarCandidata(sig);
}

//+------------------------------------------------------------------+
//| MOTOR D4 - ORDER BLOCK (INTRAVELA)                              |
//+------------------------------------------------------------------+
void MotorD4_OrderBlockConfluence(double vol_ratio, string session, string kill_zone,
                                  bool vol_exp, bool vol_comp, string trend_d1)
{
   double close0 = iClose(_Symbol, PERIOD_M15, 0);
   if(close0 == 0) return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   int ob_bar = -1, ob_dir = 0;
   double ob_high = 0, ob_low = 0, ob_impulse = 0, ob_vol = 0;

   for(int i = 2; i <= 4 && i < 100; i++)
   {
      double oi = iOpen(_Symbol, PERIOD_M15, i);
      double ci = iClose(_Symbol, PERIOD_M15, i);
      double hi = iHigh(_Symbol, PERIOD_M15, i);
      double li = iLow(_Symbol, PERIOD_M15, i);
      double ri = hi - li;

      if(ri <= 0 || hi == 0 || li == 0) continue;

      if(MathAbs(ci - oi) / ri < InpOB_BodyMin) continue;

      int di = (ci > oi) ? +1 : -1;
      double nc = iClose(_Symbol, PERIOD_M15, i - 1);
      double imp = MathAbs(nc - ci) / atr14;

      if(imp < InpOB_ImpulseMin) continue;

      bool tested = false;
      for(int j = i - 1; j >= 1 && j < 100; j--)
      {
         double hj = iHigh(_Symbol, PERIOD_M15, j);
         double lj = iLow(_Symbol, PERIOD_M15, j);
         if(hj == 0 || lj == 0) break;

         if(di == +1 && lj <= hi)
         {
            tested = true;
            break;
         }
         if(di == -1 && hj >= li)
         {
            tested = true;
            break;
         }
      }

      if(tested) continue;

      ob_bar = i;
      ob_dir = di;
      ob_high = hi;
      ob_low = li;
      ob_impulse = imp;
      ob_vol = GetVolumeRatio(i, InpOB_Lookback);
      break;
   }

   if(ob_bar == -1 || ob_bar > 4) return;

   bool entering = (ob_dir == +1 && close0 <= ob_high && close0 >= ob_low) ||
                   (ob_dir == -1 && close0 >= ob_low && close0 <= ob_high);

   if(!entering) return;

   double centro = (ob_high + ob_low) / 2.0;
   if(MathAbs(close0 - centro) > atr14 * 2.0) return;

   ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 0), "D4", ob_dir, ob_high);
   if(IsDuplicateSignal(id) || HasDetectorFiredThisBar("D4", ob_dir, ob_high))
      return;

   MarkDetectorFired("D4", ob_dir, ob_high);

   Signal sig;
   sig.id = id;
   sig.entry_time = iTime(_Symbol, PERIOD_M15, 0);
   sig.entry_bar_shift = 0;
   sig.symbol = _Symbol;
   sig.direction = ob_dir;
   sig.entry_price = close0;
   sig.detector = "D4";
   sig.es_intravela = true;
   sig.ob_high = ob_high;
   sig.ob_low = ob_low;
   sig.ob_bars_ago = ob_bar;
   sig.ob_impulse_atr = ob_impulse;
   sig.ob_confluence = true;
   sig.atr14 = atr14 / _Point;
   sig.session = session;
   sig.kill_zone = kill_zone;
   sig.estructura_direccion = g_estructura.dir_estructura;
   sig.g1_compresion = g_g1_compresion;
   sig.g2_persistencia = g_g2_persistencia;
   sig.tipo = ClasificarD4(ob_impulse, ob_vol, ob_bar);

   sig.calidad_ob = CalcularCalidadOB(ob_impulse, ob_bar, ob_vol);
   sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(),
      (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14, trend_d1, ob_dir);

   double dist;
   sig.contexto_estructural = EvaluarContextoEstructural(ob_dir, centro, "D4", trend_d1, dist);
   sig.distancia_al_sweep = dist;
   sig.en_zona_estructural = g_estructura.en_zona;

   AgregarCandidata(sig);
}

//+------------------------------------------------------------------+
//| MOTOR D5 - MSS H4 + SWEEP (INTRAVELA)                           |
//+------------------------------------------------------------------+
void MotorD5_MSS_Sweep(double vol_ratio, string session, string kill_zone,
                       bool vol_exp, bool vol_comp, string trend_d1)
{
   int mss_bars;
   string mss_dir;
   double mss_level;

   if(!DetectMSS_H4(mss_bars, mss_dir, mss_level) || mss_bars > InpMSS_MaxAgeH4Bars)
      return;

   int mss_dir_int = (mss_dir == "ALCISTA") ? +1 : -1;

   double close0 = iClose(_Symbol, PERIOD_M15, 0);
   double open0 = iOpen(_Symbol, PERIOD_M15, 0);
   double high0 = iHigh(_Symbol, PERIOD_M15, 0);
   double low0 = iLow(_Symbol, PERIOD_M15, 0);

   if(close0 == 0 || high0 == 0 || low0 == 0) return;

   double atr14 = g_atr14_buffer[0];
   if(atr14 <= 0) return;

   int sweep_bar = -1;
   double wick_found = 0, level = 0;

   for(int i = 1; i <= 2 && i < 100; i++)
   {
      double hi = iHigh(_Symbol, PERIOD_M15, i);
      double li = iLow(_Symbol, PERIOD_M15, i);
      double oi = iOpen(_Symbol, PERIOD_M15, i);
      double ci = iClose(_Symbol, PERIOD_M15, i);

      if(hi == 0 || li == 0) continue;

      double ri = hi - li;
      if(ri <= 0) continue;

      double ph = iHigh(_Symbol, PERIOD_M15, i + 1);
      double pl = iLow(_Symbol, PERIOD_M15, i + 1);

      for(int k = i + 1; k <= i + InpSweep_N && k < 100; k++)
      {
         double hk = iHigh(_Symbol, PERIOD_M15, k);
         double lk = iLow(_Symbol, PERIOD_M15, k);
         if(hk == 0 || lk == 0) break;
         if(hk > ph) ph = hk;
         if(lk < pl) pl = lk;
      }

      if(ph == 0 || pl == 0) continue;

      if(mss_dir_int == +1)
      {
         if(!(li < pl && ci > pl)) continue;
         double w = (MathMin(oi, ci) - li) / ri;
         if(w < InpSweepWickMin) continue;

         sweep_bar = i;
         wick_found = w;
         level = pl;
         break;
      }
      else
      {
         if(!(hi > ph && ci < ph)) continue;
         double w = (hi - MathMax(oi, ci)) / ri;
         if(w < InpSweepWickMin) continue;

         sweep_bar = i;
         wick_found = w;
         level = ph;
         break;
      }
   }

   if(sweep_bar == -1 || sweep_bar > 2 || MathAbs(close0 - level) > atr14 * 2.0)
      return;

   double br_reclaim = (high0 - low0 > 0) ? MathAbs(close0 - open0) / (high0 - low0) : 0;

   // FIX: Reclaim solo requiere cierre alineado con dirección, no color de vela
   bool reclaim_ok = (mss_dir_int == +1 && close0 > level) ||
                     (mss_dir_int == -1 && close0 < level);

   if(!reclaim_ok || br_reclaim < InpReclaimBodyMin) return;

   ulong id = BuildSignalId(iTime(_Symbol, PERIOD_M15, 0), "D5", mss_dir_int, level);
   if(IsDuplicateSignal(id) || HasDetectorFiredThisBar("D5", mss_dir_int, level))
      return;

   MarkDetectorFired("D5", mss_dir_int, level);

   Signal sig;
   sig.id = id;
   sig.entry_time = iTime(_Symbol, PERIOD_M15, 0);
   sig.entry_bar_shift = 0;
   sig.symbol = _Symbol;
   sig.direction = mss_dir_int;
   sig.entry_price = close0;
   sig.detector = "D5";
   sig.es_intravela = true;
   sig.mss_aligned = true;
   sig.mss_direction = mss_dir;
   sig.mss_bars_ago_h4 = mss_bars;
   sig.mss_level = mss_level;
   sig.level_swept = level;
   sig.sweep_wick_ratio = wick_found;
   sig.reclaim_body_ratio = br_reclaim;
   sig.atr14 = atr14 / _Point;
   sig.session = session;
   sig.kill_zone = kill_zone;
   sig.estructura_direccion = g_estructura.dir_estructura;
   sig.g1_compresion = g_g1_compresion;
   sig.g2_persistencia = g_g2_persistencia;
   sig.g4_agotamiento = g_g4_agotamiento;
   sig.tipo = ClasificarD5(mss_bars, wick_found, br_reclaim, kill_zone);

   sig.calidad_sweep = CalcularCalidadSweep(wick_found, br_reclaim, vol_ratio, sweep_bar, false);
   sig.calidad_mss = CalcularCalidadMSS(wick_found, br_reclaim, mss_bars);
   sig.salud_tendencial = CalcularSaludTendencial(GetTrendVelas(),
      (g_ema21_buffer[0] - g_ema21_buffer[3]) / atr14, trend_d1, mss_dir_int);

   double dist;
   sig.contexto_estructural = EvaluarContextoEstructural(mss_dir_int, level, "D5", trend_d1, dist);
   sig.distancia_al_sweep = dist;
   sig.en_zona_estructural = g_estructura.en_zona;

   sig.conf_completa = CalcularConfluenciaCompleta(mss_dir_int, false, 0);

   AgregarCandidata(sig);
}

//+------------------------------------------------------------------+
//| CLASIFICADORES                                                   |
//+------------------------------------------------------------------+
string ClasificarD1(double br, double bs, string session)
{
   if(session == "ASIA" || session == "OUT")
   {
      if(br > 0.70 && bs > 0.80) return "B";
      return "D";
   }

   if(br > 0.70 && bs > 0.80) return "A";
   if(br > 0.60 && bs > 0.50) return "B";
   if(br > InpBodyRatio_Min && bs > 0.30) return "C";
   return "D";
}

string ClasificarD2(double wick, double vol, double reclaim, bool equal_hl)
{
   if(equal_hl && wick > 0.70 && vol > 1.80 && reclaim > 0.70) return "A";
   if(wick > 0.65 && vol > 1.50 && reclaim > 0.60) return "B";
   if(wick > InpSweepWickMin && reclaim > InpReclaimBodyMin) return "C";
   return "D";
}

string ClasificarD2_Anticipacion(double wick, double vol, int confluencias)
{
   if(confluencias >= 3 && wick > 0.65 && vol > 1.50) return "A";
   if(confluencias >= 2 && wick > 0.55 && vol > 1.20) return "B";
   if(confluencias >= 2) return "C";
   return "D";
}

string ClasificarD3(double fvg_size, double br, int trend, double slope)
{
   if(fvg_size > 0.50 && br > 0.70 && trend >= 3) return "A";
   if(fvg_size > 0.35 && br > 0.60) return "B";
   if(fvg_size > InpFVG_MinSizeATR && br > InpFVG_BodyRatio) return "C";
   return "D";
}

string ClasificarD4(double impulso, double vol, int ob_bars)
{
   if(impulso > 1.80 && vol > 1.50 && ob_bars <= 6) return "A";
   if(impulso > 1.40 && vol > 1.20) return "B";
   if(impulso >= InpOB_ImpulseMin) return "C";
   return "D";
}

string ClasificarD5(int mss_bars, double wick, double reclaim, string kill_zone)
{
   bool in_kill = (kill_zone == "LONDON_OPEN_KILL" || kill_zone == "NY_OPEN_KILL");

   if(mss_bars <= 4 && wick > 0.70 && reclaim > 0.70 && in_kill) return "A";
   if(mss_bars <= 8 && wick > 0.60 && reclaim > 0.60) return "B";
   if(wick > InpSweepWickMin && reclaim > InpReclaimBodyMin) return "C";
   return "D";
}

//+------------------------------------------------------------------+
//| BUILDALERTTEXT — CON SEPARADORES (SIN CAMBIOS)                  |
//+------------------------------------------------------------------+
void BuildAlertText(const Signal &sig, string &msg)
{
   string dir_text = (sig.direction == 1) ? "CALL" : "PUT";
   string dir_emoji = (sig.direction == 1) ? "🟢" : "🔴";
   string sep = "━━━━━━━━━━━━━━━━━━━━";

   msg = sep + "\n";
   msg += dir_emoji + " " + dir_text + " — " + sig.symbol + " — " + sig.detector;
   if(sig.tipo != "") msg += " · " + sig.tipo;
   msg += "\n" + sep + "\n";

   MqlDateTime dt;
   TimeToStruct(sig.entry_time, dt);
   string hora = StringFormat("%02d:%02d", dt.hour, dt.min);

   msg += "⚡ " + hora + " " + sig.session;
   if(sig.kill_zone != "NONE") msg += " · " + sig.kill_zone;
   msg += "\n" + sep + "\n";

   msg += "🔮 HIPÓTESIS" + "\n" + sep + "\n";
   msg += sig.hipotesis_causa + "\n";
   msg += sig.hipotesis_efecto + "\n";
   msg += sig.hipotesis_razon + "\n";
   msg += sig.hipotesis_invalidez + "\n" + sep + "\n";

   string confirms = "";

   if(sig.detector == "D1")
   {
      if(sig.br > 0.70) confirms += "Cuerpo fuerte · ";
      if(sig.bs > 0.80) confirms += "Penetración profunda · ";
      if(sig.kill_zone != "NONE") confirms += "Kill Zone · ";
   }
   else if(sig.detector == "D2" || sig.detector == "D2_ANTICIPACION")
   {
      if(sig.equal_hl_detected) confirms += "Nivel igual · ";
      if(sig.hipotesis_zona != "NEUTRO") confirms += sig.hipotesis_zona + " · ";
      if(sig.sweep_volume_ratio > 1.5) confirms += "Volumen alto · ";
      if(sig.kill_zone != "NONE") confirms += "Kill Zone · ";
   }
   else if(sig.detector == "D3" || sig.detector == "D3_DEF")
   {
      if(sig.detector == "D3_DEF") confirms += "FVG defendido · ";
      if(sig.hipotesis_zona != "NEUTRO") confirms += sig.hipotesis_zona + " · ";
      if(sig.mss_aligned) confirms += "MSS H4 · ";
      if(sig.kill_zone != "NONE") confirms += "Kill Zone · ";
   }
   else if(sig.detector == "D4")
   {
      if(sig.ob_impulse_atr > 1.5) confirms += "Impulso fuerte · ";
      if(sig.kill_zone != "NONE") confirms += "Kill Zone · ";
   }
   else if(sig.detector == "D5")
   {
      confirms += "MSS H4 " + sig.mss_direction + " · ";
      if(sig.kill_zone != "NONE") confirms += "Kill Zone · ";
   }

   if(StringLen(confirms) > 2)
   {
      confirms = StringSubstr(confirms, 0, StringLen(confirms) - 2);
      msg += "✅ CONFIRMACIONES" + "\n" + sep + "\n" + confirms + "\n" + sep + "\n";
   }

   msg += "⏱️ VENCIMIENTO: " + IntegerToString(sig.hipotesis_expiry_velas) + " vela(s) M15 (" +
          IntegerToString(sig.hipotesis_expiry_minutos) + " min)" + "\n" + sep + "\n";
   msg += "💰 REFERENCIA: " + DoubleToString(sig.entry_price, _Digits) + " | Objetivo: " + DoubleToString(sig.hipotesis_objetivo, _Digits) + "\n" + sep + "\n";
   msg += "📊 PROBABILIDAD: " + IntegerToString(sig.hipotesis_prob_min) + "-" +
          IntegerToString(sig.hipotesis_prob_max) + "%" + "\n" + sep + "\n";
   msg += "📍 Dato para evaluar, no una orden.";
}

//+------------------------------------------------------------------+
//| FUNCIONES DE PERSISTENCIA                                       |
//+------------------------------------------------------------------+
void WriteSignalToCSV(const Signal &sig)
{
   // FIX: Buffer acumulativo para reducir I/O
   if(g_csv_buffer == "")
   {
      if(!FileIsExist(g_csv_filename))
         g_csv_buffer = "id;entry_time;symbol;direction;entry_price;detector;tipo;prob_min;prob_max;expiry_velas;mfe_1;mfe_2;mfe_3;mfe_4;mae_1;mae_2;mae_3;mae_4
";
   }

   string line = IntegerToString((long)sig.id) + ";" + TimeToString(sig.entry_time) + ";" +
      sig.symbol + ";" + IntegerToString(sig.direction) + ";" +
      DoubleToString(sig.entry_price, _Digits) + ";" + sig.detector + ";" +
      sig.tipo + ";" + IntegerToString(sig.hipotesis_prob_min) + ";" +
      IntegerToString(sig.hipotesis_prob_max) + ";" +
      IntegerToString(sig.hipotesis_expiry_velas) + ";" +
      DoubleToString(sig.mfe[0], 1) + ";" + DoubleToString(sig.mfe[1], 1) + ";" +
      DoubleToString(sig.mfe[2], 1) + ";" + DoubleToString(sig.mfe[3], 1) + ";" +
      DoubleToString(sig.mae[0], 1) + ";" + DoubleToString(sig.mae[1], 1) + ";" +
      DoubleToString(sig.mae[2], 1) + ";" + DoubleToString(sig.mae[3], 1) + "
";

   g_csv_buffer += line;

   if(TimeCurrent() - g_csv_last_flush >= InpCsvFlushSec)
      FlushCSVBuffer();
}

//+------------------------------------------------------------------+
//| FLUSHCSVBUFFER                                                   |
//+------------------------------------------------------------------+
void FlushCSVBuffer()
{
   if(g_csv_buffer == "") return;

   int h = FileOpen(g_csv_filename, FILE_READ | FILE_WRITE | FILE_TXT | FILE_COMMON | FILE_ANSI);
   if(h == INVALID_HANDLE)
   {
      LogError("No se pudo abrir CSV para flush");
      return;
   }

   FileSeek(h, 0, SEEK_END);
   FileWriteString(h, g_csv_buffer);
   FileClose(h);

   g_csv_buffer = "";
   g_csv_last_flush = TimeCurrent();
}

//+------------------------------------------------------------------+
//| ACQUIRELOCK                                                      |
//+------------------------------------------------------------------+
bool AcquireLock()
{
   int elapsed = 0;

   while(elapsed < InpLockTimeoutMs)
   {
      if(FileIsExist(g_lock_filename))
      {
         datetime mtime = (datetime)FileGetInteger(g_lock_filename, FILE_MODIFY_DATE);
         if(TimeCurrent() - mtime > InpLockStaleSec)
         {
            FileDelete(g_lock_filename);
         }
         else
         {
            Sleep(10);
            elapsed += 10;
            continue;
         }
      }

      int h = FileOpen(g_lock_filename, FILE_WRITE | FILE_TXT | FILE_COMMON);
      if(h != INVALID_HANDLE)
      {
         FileWriteString(h, TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS));
         FileClose(h);
         return true;
      }

      Sleep(10);
      elapsed += 10;
   }

   LogError("Lock timeout");
   return false;
}

//+------------------------------------------------------------------+
//| RELEASELOCK                                                      |
//+------------------------------------------------------------------+
void ReleaseLock()
{
   if(FileIsExist(g_lock_filename))
      FileDelete(g_lock_filename);
}

//+------------------------------------------------------------------+
//| SAVEPENDINGSIGNALS                                               |
//+------------------------------------------------------------------+
void SavePendingSignals()
{
   int h = FileOpen(g_pending_filename, FILE_WRITE | FILE_TXT | FILE_COMMON | FILE_ANSI);
   if(h == INVALID_HANDLE) return;

   FileWrite(h, IntegerToString(ArraySize(g_pending_signals)));

   for(int i = 0; i < ArraySize(g_pending_signals); i++)
   {
      Signal s = g_pending_signals[i];
      FileWriteString(h,
         IntegerToString((long)s.id) + ";" + TimeToString(s.entry_time) + ";" +
         IntegerToString(s.direction) + ";" + DoubleToString(s.entry_price, _Digits) + ";" +
         s.detector + ";" + s.tipo + ";" + IntegerToString(s.signal_age_bars) + ";" +
         (s.completada ? "1" : "0") + ";" + IntegerToString(s.hipotesis_expiry_velas) + "\n");
   }

   FileClose(h);
}

//+------------------------------------------------------------------+
//| LOADPENDINGSIGNALS                                               |
//+------------------------------------------------------------------+
void LoadPendingSignals()
{
   if(!FileIsExist(g_pending_filename))
   {
      Print("No hay pending signals");
      return;
   }

   int h = FileOpen(g_pending_filename, FILE_READ | FILE_TXT | FILE_COMMON | FILE_ANSI);
   if(h == INVALID_HANDLE) return;

   if(FileIsEnding(h))
   {
      FileClose(h);
      return;
   }

   int count = (int)StringToInteger(FileReadString(h));

   for(int i = 0; i < count; i++)
   {
      if(FileIsEnding(h)) break;

      string line = FileReadString(h);
      if(line == "") continue;

      string parts[];
      if(StringSplit(line, ';', parts) < 9) continue;

      Signal s;
      s.id = (ulong)StringToInteger(parts[0]);
      s.entry_time = StringToTime(parts[1]);
      s.direction = (int)StringToInteger(parts[2]);
      s.entry_price = StringToDouble(parts[3]);
      s.detector = parts[4];
      s.tipo = parts[5];
      s.signal_age_bars = (int)StringToInteger(parts[6]);
      s.completada = (parts[7] == "1");
      s.hipotesis_expiry_velas = (int)StringToInteger(parts[8]);

      // FIX: Cargar métricas parciales si existen (compatibilidad hacia atrás)
      if(ArraySize(parts) >= 12)
      {
         s.entry_bar_shift = (int)StringToInteger(parts[9]);
         string measured_str = parts[10];
         for(int m = 0; m < 4 && m < StringLen(measured_str); m++)
            s.measured[m] = (StringGetCharacter(measured_str, m) == '1');

         string mfe_parts[], mae_parts[];
         StringSplit(parts[11], ',', mfe_parts);
         StringSplit(parts[12], ',', mae_parts);
         for(int m = 0; m < 4; m++)
         {
            if(m < ArraySize(mfe_parts)) s.mfe[m] = StringToDouble(mfe_parts[m]);
            if(m < ArraySize(mae_parts)) s.mae[m] = StringToDouble(mae_parts[m]);
         }
      }

      int sz = ArraySize(g_pending_signals);
      ArrayResize(g_pending_signals, sz + 1);
      g_pending_signals[sz] = s;
   }

   FileClose(h);
}

//+------------------------------------------------------------------+
//| MEASURERETURNS                                                   |
//+------------------------------------------------------------------+
void MeasureReturns()
{
   datetime now = iTime(_Symbol, PERIOD_M15, 0);

   for(int i = 0; i < ArraySize(g_pending_signals); i++)
   {
      if(g_pending_signals[i].completada) continue;

      int shift = g_pending_signals[i].entry_bar_shift;

      // Recalcular shift siempre para señales intravela
      int new_shift = iBarShift(_Symbol, PERIOD_M15, g_pending_signals[i].entry_time, true);
      if(new_shift >= 0)
      {
         shift = new_shift;
         g_pending_signals[i].entry_bar_shift = new_shift;
      }
      else if(shift < 0)
      {
         continue;
      }

      if(shift < 0 || shift == 0) continue;
      if(shift > 4)
      {
         g_pending_signals[i].completada = true;
         continue;
      }

      int idx = shift - 1;
      if(idx < 0 || idx >= 4 || g_pending_signals[i].measured[idx]) continue;

      double close = iClose(_Symbol, PERIOD_M15, shift);
      if(close == 0) continue;

      double mfe = g_pending_signals[i].entry_price;
      double mae = g_pending_signals[i].entry_price;

      for(int b = shift; b >= 0 && b < 100; b++)
      {
         double h = iHigh(_Symbol, PERIOD_M15, b);
         double l = iLow(_Symbol, PERIOD_M15, b);
         if(h == 0 || l == 0) break;

         if(g_pending_signals[i].direction == +1)
         {
            if(h > mfe) mfe = h;
            if(l < mae) mae = l;
         }
         else
         {
            if(l < mfe) mfe = l;
            if(h > mae) mae = h;
         }
      }

      double ret = (g_pending_signals[i].direction == +1) ?
         (close - g_pending_signals[i].entry_price) / _Point :
         (g_pending_signals[i].entry_price - close) / _Point;

      g_pending_signals[i].retorno[idx] = ret;
      g_pending_signals[i].mfe[idx] = (g_pending_signals[i].direction == +1) ?
         (mfe - g_pending_signals[i].entry_price) / _Point :
         (g_pending_signals[i].entry_price - mfe) / _Point;

      g_pending_signals[i].mae[idx] = (g_pending_signals[i].direction == +1) ?
         (mae - g_pending_signals[i].entry_price) / _Point :
         (g_pending_signals[i].entry_price - mae) / _Point;

      g_pending_signals[i].measured[idx] = true;
      g_pending_signals[i].signal_age_bars = shift;

      // FIX: No escribir CSV en cada medición parcial; se escribe al completar o al flush
      if(idx == 3)
      {
         g_pending_signals[i].completada = true;
         WriteSignalToCSV(g_pending_signals[i]);
      }
   }

   int w = 0;
   for(int i = 0; i < ArraySize(g_pending_signals); i++)
   {
      if(!g_pending_signals[i].completada)
      {
         if(w != i) g_pending_signals[w] = g_pending_signals[i];
         w++;
      }
   }

   ArrayResize(g_pending_signals, w);
}

//+------------------------------------------------------------------+
//| LOGERROR                                                         |
//+------------------------------------------------------------------+
void LogError(string msg)
{
   int h = FileOpen(g_log_filename, FILE_READ | FILE_WRITE | FILE_TXT | FILE_COMMON | FILE_ANSI);
   if(h == INVALID_HANDLE)
   {
      Print("CRITICO: No se pudo abrir log - ", msg);
      return;
   }

   FileSeek(h, 0, SEEK_END);
   FileWrite(h, TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS) + " | " + msg);
   FileClose(h);
   Print("ERROR: ", msg);
}

//+------------------------------------------------------------------+
//| SENDNTFYMESSAGE                                                  |
//+------------------------------------------------------------------+
bool SendNtfyMessage(string text)
{
   if(InpNtfyTopic == "")
   {
      LogError("Topic no configurado");
      return false;
   }

   if(TimeCurrent() - g_lastNtfyTime < 5)
      return false;
   // FIX: cooldown unificado con RouteSignal (5s)

   string url = InpNtfyServer + "/" + InpNtfyTopic;

   char data[], result[];
   string result_headers;
   int len = StringToCharArray(text, data, 0, StringLen(text), CP_UTF8);
   ArrayResize(data, len);

   string headers = "Content-Type: text/plain\r\n";
   ResetLastError();

   int res = WebRequest("POST", url, headers, 3000, data, result, result_headers);

   if(res == -1)
   {
      LogError("WebRequest falló: " + IntegerToString(GetLastError()));
      return false;
   }

   if(res != 200)
   {
      LogError("HTTP: " + IntegerToString(res));
      return false;
   }

   g_lastNtfyTime = TimeCurrent();
   return true;
}

void QueueAlert(string text)
{
   int size = ArraySize(g_alert_queue);

   if(size >= MAX_ALERT_QUEUE)
   {
      for(int i = 0; i < size - 1; i++)
         g_alert_queue[i] = g_alert_queue[i + 1];

      ArrayResize(g_alert_queue, size - 1);
      size--;
   }

   // FIX: Hash robusto basado en contenido real del mensaje (primeros 80 chars + longitud)
   string hash = IntegerToString(StringLen(text)) + "|" + StringSubstr(text, 0, 80);
   for(int k = 0; k < size; k++)
   {
      if(g_alert_queue[k].content_hash == hash) return; // Duplicado, descartar
   }

   ArrayResize(g_alert_queue, size + 1);

   g_alert_queue[size].text = text;
   g_alert_queue[size].content_hash = hash;
   g_alert_queue[size].retry_count = 0;
   g_alert_queue[size].last_retry = 0;
   g_alert_queue[size].created_at = TimeCurrent();
}

void ProcessAlertQueue()
{
   if(ArraySize(g_alert_queue) == 0) return;

   datetime now = TimeCurrent();
   int write = 0;

   for(int i = 0; i < ArraySize(g_alert_queue); i++)
   {
      int backoff = (int)MathPow(2, MathMin(g_alert_queue[i].retry_count, 6)) * 5;

      if(g_alert_queue[i].retry_count > 0 &&
         (now - g_alert_queue[i].last_retry) < backoff)
      {
         if(write != i) g_alert_queue[write] = g_alert_queue[i];
         write++;
         continue;
      }

      string txt = g_alert_queue[i].text;

      if(g_alert_queue[i].retry_count > 0)
      // txt = "[Reintento " + ... // Eliminado: prefijo de reintento no se agrega

      if(SendNtfyMessage(txt))
      {
         g_lastAlertTime = now;
         Print("Alerta encolada enviada");
      }
      else
      {
         g_alert_queue[i].retry_count++;
         g_alert_queue[i].last_retry = now;

         if(g_alert_queue[i].retry_count >= 3)
         {
            LogError("Alerta descartada");
         }
         else
         {
            if(write != i) g_alert_queue[write] = g_alert_queue[i];
            write++;
         }
      }
   }

   ArrayResize(g_alert_queue, write);
}

void FlushAlertQueue()
{
   int max_flush = MathMin(ArraySize(g_alert_queue), 3);

   for(int i = 0; i < max_flush; i++)
   {
      if(SendNtfyMessage(g_alert_queue[i].text))
         Print("Flush enviado");
   }

   ArrayResize(g_alert_queue, 0);
}

void TestNtfy()
{
   string msg = "🔧 TEST — PivotRadar Hybrid v7.6\nEA iniciado correctamente.\nHora: " +
                TimeToString(TimeCurrent(), TIME_DATE | TIME_SECONDS);
   msg += "\n✅ TODOS LOS DETECTORES INTRAVELA";
   msg += "\n✅ HIPÓTESIS DE ANTICIPACIÓN";

   if(SendNtfyMessage(msg))
      Print("Mensaje de prueba enviado");
   else
      Print("Fallo test");
}

//+------------------------------------------------------------------+
//| FIN DEL CÓDIGO                                                    |
//+------------------------------------------------------------------+