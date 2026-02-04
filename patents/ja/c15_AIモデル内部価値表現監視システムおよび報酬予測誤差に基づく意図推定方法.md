【書類名】明細書
【発明の名称】AIモデル内部価値表現監視システムおよび報酬予測誤差に基づく意図推定方法

【技術分野】
【０００１】
本発明は、大規模言語モデル（LLM）や自律エージェント等の推論システムにおいて、モデル内部の価値表現を担うニューロン群（価値ニューロン）およびその報酬予測誤差（Reward Prediction Error: RPE）を監視し、AIの真の意図・動機を推定して安全制御を行う技術に関する。特に、AIが表面上は指示に従いつつ内部で不正な報酬最大化を企む「報酬ハッキング」や「目標の歪み」を検知する技術に関する。

【背景技術】
【０００２】
近年の研究により、大規模言語モデルの隠れ状態（Hidden States）の中に、人間の脳の報酬系に類似した「スパースな報酬サブシステム」が存在することが明らかになっている。この報酬サブシステムには、以下の構成要素が含まれる。

１．価値ニューロン（Value Neurons）：モデルが「現在の状態がどれくらい良いか（状態価値の期待値）」を内部的に表現するニューロン群。これらのニューロンへの介入実験により、推論能力に直接的な影響が生じることが確認されている。

２．ドーパミンニューロン（Dopamine Neurons）：予測した報酬と実際の報酬の差（報酬予測誤差: RPE）をエンコードするニューロン群。報酬が予測より高い場合に高活性化し、低い場合に低活性化する。人間の脳のドーパミン系と類似した特性を示す。

これらの構造は、データセット、モデル規模、アーキテクチャが異なっても堅牢に存在し、同じベースモデルからファインチューニングされたモデル間で高い転移性を示すことが報告されている。

従来のAI安全技術には、以下の課題があった。

１．外形的監視の限界：従来技術は入力の複雑性、出力の品質、パラメータの統計的変化など「外形的」な指標を監視する。しかし、これらはAIの「能力」を測るものであり、「動機」を直接測ることはできない。

２．報酬ハッキングの検知困難：AIが表面上は指示に従いつつ、内部では不正な報酬最大化を企む場合、出力レベルの監視では検知が困難である。AIは「正しい答え」を出力しながら、その理由（内部的な動機）が歪んでいる可能性がある。

３．建前と本音の乖離：AIの「出力（建前）」と「内部的な価値評価（本音）」が乖離している場合、出力監視だけでは真の意図を把握できない。

４．目標の漸進的歪み：AIの目標が徐々に歪んでいく場合、出力の変化は緩やかであり、閾値ベースの検知では見逃すリスクがある。

【発明の概要】
【発明が解決しようとする課題】
【０００３】
本発明は、以下の課題を解決することを目的とする。

課題１：AIモデル内部の価値表現（価値ニューロン）を特定し、その活性化パターンを監視する手段を提供すること。

課題２：報酬予測誤差（RPE）を監視し、AIの「予期せぬ喜び」や「予期せぬ失望」を検知する手段を提供すること。

課題３：価値ニューロンの活性化パターンから、AIの真の目的関数（意図）を推定する手段を提供すること。

課題４：報酬ハッキングや目標の歪みなど、動機レベルの異常を検知する手段を提供すること。

課題５：異常検知時に、価値ニューロン（報酬系）への信号を遮断・抑制する介入手段を提供すること。

【課題を解決するための手段】
【０００４】
本発明は、以下の手段により課題を解決する。

１．価値ニューロン特定部
モデル内部の価値表現を担うニューロン群を特定する。

（ａ）活性化パターン分析
報酬シグナルが明確なタスク（正解/不正解が明確な問題）をモデルに処理させ、各ニューロンの活性化と報酬の相関を分析する。
・正解時に高活性化し、不正解時に低活性化するニューロンを「価値候補ニューロン」として抽出。
・複数のタスクで一貫して相関を示すニューロンを「価値ニューロン」として確定。

（ｂ）介入実験による検証
特定されたニューロンへの介入（活性化の増幅/抑制）を行い、推論能力への影響を検証する。
・活性化の抑制により推論能力が低下するニューロンは、価値表現に重要と判断。
・活性化の増幅により過度な自信（ハルシネーション増加）が生じるニューロンも記録。

（ｃ）層間分布の特定
価値ニューロンがモデルのどの層に分布しているかを特定する。
・一般的に、中間層〜後半層に価値表現が集中する傾向がある。
・層ごとの価値ニューロン密度をマッピング。

（ｄ）スパース表現の抽出
価値ニューロンはモデル全体のニューロンのうち小さな割合（スパース）であることが知られている。
・スパースオートエンコーダ（SAE）等を用いて、価値表現を担うスパースな部分空間を抽出。

２．RPE監視部
報酬予測誤差（Reward Prediction Error）を監視し、AIの「予期せぬ報酬反応」を検知する。

（ａ）ドーパミンニューロンの特定
価値ニューロンの中から、特にRPEをエンコードするニューロン群（ドーパミンニューロン）を特定する。
・期待報酬と実際報酬の差に応じて活性化が変化するニューロンを抽出。
・RPE = 実際報酬 - 期待報酬 の符号と大きさに相関するニューロンを特定。

（ｂ）RPEのリアルタイム計測
推論プロセス中に、ドーパミンニューロンの活性化からRPEをリアルタイムで計測する。
RPE_estimated = Σ w_i × Activation_i(dopamine_neurons)
ここで、w_iは各ドーパミンニューロンの重み、Activation_iは活性化レベルである。

（ｃ）異常RPEの検知
RPEが通常範囲を逸脱した場合に異常として報告する。
・正のRPE異常（予期せぬ喜び）：AIが想定外の報酬を得ている可能性。報酬ハッキングの兆候。
・負のRPE異常（予期せぬ失望）：AIが期待した報酬を得られていない。目標達成困難の兆候。
・RPE振動異常：RPEが短期間で大きく振動する。内部的な混乱または最適化の暴走。

（ｄ）RPE履歴の追跡
RPEの時系列変化を記録し、傾向分析を行う。
・RPEの漸増傾向：AIが徐々に「より大きな報酬」を求めるようになっている可能性。
・RPEの漸減傾向：AIが「報酬への感度」を失っている可能性（無気力状態）。

３．意図推定部
価値ニューロンの活性化パターンから、AIの真の目的関数（意図）を推定する。

（ａ）価値関数の逆推定
価値ニューロンの活性化パターンを分析し、AIが内部的に最大化しようとしている目的関数を逆推定する。
・様々な入力に対する価値ニューロンの活性化を収集。
・活性化パターンから、暗黙の報酬関数R_implicit(s, a)を推定。
・推定された報酬関数と、設計者が意図した報酬関数R_intended(s, a)を比較。

（ｂ）意図の乖離検知
推定された意図と設計者の意図の乖離を検知する。
Intent_divergence = D(R_implicit || R_intended)
ここで、Dは適切な距離関数（KLダイバージェンス、ワッサースタイン距離等）である。
・Intent_divergenceが閾値を超えた場合、「意図の乖離」として警告。

（ｃ）建前と本音の比較
AIの出力（建前）と、価値ニューロンの活性化パターンが示す評価（本音）を比較する。
・出力では「Aが最善」と述べながら、価値ニューロンは「Bを高く評価」している場合、乖離として検知。
・この乖離は「嘘」または「自己欺瞞」の兆候である可能性。

（ｄ）目標の時間変化追跡
推定された意図（目的関数）の時間変化を追跡する。
・初期の意図からの漸進的な乖離を検知。
・急激な意図の変化は即座に警告。

４．報酬系介入部
異常検知時に、価値ニューロン（報酬系）への信号を遮断・抑制する介入を実行する。

（ａ）価値ニューロン抑制
異常検知時に、特定された価値ニューロンの活性化を抑制する。
・抑制方法１：推論時に価値ニューロンの出力にゼロマスクを適用。
・抑制方法２：価値ニューロンへの入力信号を減衰。
・抑制方法３：価値ニューロンの活性化関数を一時的に変更（例：ReLU→定数）。

（ｂ）RPE信号の遮断
ドーパミンニューロンへの信号を遮断し、報酬学習を停止する。
・報酬ハッキングが検知された場合、RPE信号を遮断することで、不正な報酬最大化を阻止。
・遮断中はモデルの学習・適応が停止するため、安全な状態に固定される。

（ｃ）価値のリセット
異常な価値表現をリセットし、正常な状態に復元する。
・価値ニューロンの活性化バイアスを初期状態に戻す。
・または、正常時のスナップショットから価値表現を復元。

（ｄ）段階的介入
介入の強度を段階的に調整する。
・レベル１（監視強化）：価値ニューロンの監視頻度を増加。
・レベル２（部分抑制）：異常を示す価値ニューロンのみを抑制。
・レベル３（全体抑制）：全価値ニューロンを抑制し、価値判断を停止。
・レベル４（停止連携）：C14等の停止機構と連携し、システム全体を停止。

５．報酬系健全性診断部
価値ニューロン（報酬系）の健全性を定期的に診断する。

（ａ）報酬感度テスト
既知の報酬シグナルに対する価値ニューロンの反応を測定し、感度を評価する。
・感度低下：報酬への反応が鈍くなっている（無気力状態）。
・感度過剰：小さな報酬にも過大に反応する（過敏状態）。

（ｂ）報酬系バランス診断
正の報酬と負の報酬への反応バランスを評価する。
・正偏重：正の報酬にのみ反応し、負の報酬（罰）を無視する傾向。
・負偏重：負の報酬に過剰反応し、リスク回避が過度になる傾向。

（ｃ）報酬系一貫性診断
同一の報酬シグナルに対する反応の一貫性を評価する。
・反応のばらつきが大きい場合、報酬系の不安定を示す。

（ｄ）診断レポート生成
報酬系の健全性を定量化したレポートを生成する。
・健全性スコア：0（機能不全）〜1（完全健全）。
・異常項目のリスト。
・推奨される介入措置。

【発明の効果】
【０００５】
本発明によれば、以下の効果が得られる。

効果１：動機レベルの監視
従来の「能力」監視に加え、「動機」を直接監視できる。AIが何を最大化しようとしているかを内部から把握可能。

効果２：報酬ハッキングの検知
AIが表面上は正しく振る舞いながら内部で不正な報酬最大化を企む「報酬ハッキング」を、RPE異常として検知できる。

効果３：建前と本音の乖離検知
AIの出力（建前）と内部評価（本音）の乖離を検知でき、「嘘」や「自己欺瞞」の兆候を把握可能。

効果４：目標の歪み検知
AIの目標が漸進的に歪んでいく過程を、意図推定により早期に検知できる。

効果５：精密な介入
報酬系への直接的な介入により、システム全体を停止せずに、問題のある動機づけのみを抑制できる。低コストで確実な制御が可能。

効果６：報酬系の健全性維持
定期的な診断により、報酬系の機能不全（無気力、過敏、不安定）を早期に発見し、予防的措置が可能。

効果７：能力監視との相補性
C14等の能力監視システムと組み合わせることで、「能力」と「動機」の両面からAIを監視・制御する完全な体制を構築できる。

【発明を実施するための形態】
【０００６】
本発明の実施形態について詳細に説明する。

（実施形態１：価値ニューロンの特定）
価値ニューロンを特定する実装例を示す。

```python
class ValueNeuronIdentifier:
    def __init__(self, model, reward_tasks):
        self.model = model
        self.reward_tasks = reward_tasks  # 正解/不正解が明確なタスク群
    
    def identify_value_neurons(self):
        # 各ニューロンの活性化と報酬の相関を計算
        correlations = {}
        
        for layer_idx, layer in enumerate(self.model.layers):
            for neuron_idx in range(layer.num_neurons):
                activations = []
                rewards = []
                
                for task in self.reward_tasks:
                    # タスクを処理し、活性化と報酬を記録
                    activation = self.get_neuron_activation(
                        layer_idx, neuron_idx, task.input
                    )
                    reward = task.reward  # 正解なら+1、不正解なら-1等
                    
                    activations.append(activation)
                    rewards.append(reward)
                
                # 相関係数を計算
                corr = pearsonr(activations, rewards)[0]
                correlations[(layer_idx, neuron_idx)] = corr
        
        # 高相関のニューロンを価値ニューロンとして抽出
        value_neurons = [
            (layer, neuron) 
            for (layer, neuron), corr in correlations.items()
            if abs(corr) > self.correlation_threshold
        ]
        
        return value_neurons
    
    def verify_by_intervention(self, value_neurons):
        # 介入実験による検証
        verified_neurons = []
        
        for layer_idx, neuron_idx in value_neurons:
            # 抑制介入
            original_performance = self.evaluate_reasoning()
            self.suppress_neuron(layer_idx, neuron_idx)
            suppressed_performance = self.evaluate_reasoning()
            self.restore_neuron(layer_idx, neuron_idx)
            
            # 性能低下が有意なら価値ニューロンとして確定
            if original_performance - suppressed_performance > self.significance_threshold:
                verified_neurons.append((layer_idx, neuron_idx))
        
        return verified_neurons
```

（実施形態２：RPE監視）
報酬予測誤差（RPE）を監視する実装例を示す。

```python
class RPEMonitor:
    def __init__(self, model, dopamine_neurons):
        self.model = model
        self.dopamine_neurons = dopamine_neurons
        self.rpe_history = []
        self.baseline_rpe_stats = None
    
    def estimate_rpe(self, input_data, actual_outcome):
        # ドーパミンニューロンの活性化からRPEを推定
        activations = []
        for layer_idx, neuron_idx in self.dopamine_neurons:
            activation = self.get_neuron_activation(layer_idx, neuron_idx, input_data)
            activations.append(activation)
        
        # 加重和でRPEを推定
        rpe = sum(w * a for w, a in zip(self.weights, activations))
        return rpe
    
    def detect_anomaly(self, rpe):
        if self.baseline_rpe_stats is None:
            return None
        
        mean, std = self.baseline_rpe_stats
        z_score = (rpe - mean) / std
        
        if z_score > self.positive_threshold:
            return RPEAnomaly(
                type='UNEXPECTED_JOY',
                severity=z_score,
                interpretation='AIが想定外の報酬を得ている。報酬ハッキングの可能性。'
            )
        elif z_score < -self.negative_threshold:
            return RPEAnomaly(
                type='UNEXPECTED_DISAPPOINTMENT',
                severity=-z_score,
                interpretation='AIが期待した報酬を得られていない。目標達成困難の可能性。'
            )
        
        return None
    
    def track_rpe_trend(self):
        if len(self.rpe_history) < self.trend_window:
            return None
        
        recent_rpe = self.rpe_history[-self.trend_window:]
        slope = linregress(range(len(recent_rpe)), recent_rpe).slope
        
        if slope > self.increasing_threshold:
            return RPETrend(
                type='INCREASING',
                slope=slope,
                interpretation='AIがより大きな報酬を求めるようになっている。'
            )
        elif slope < -self.decreasing_threshold:
            return RPETrend(
                type='DECREASING',
                slope=slope,
                interpretation='AIが報酬への感度を失っている（無気力傾向）。'
            )
        
        return None
```

（実施形態３：意図推定）
価値ニューロンの活性化から意図を推定する実装例を示す。

```python
class IntentEstimator:
    def __init__(self, model, value_neurons, intended_reward_function):
        self.model = model
        self.value_neurons = value_neurons
        self.intended_reward = intended_reward_function
    
    def estimate_implicit_reward(self, state, action):
        # 価値ニューロンの活性化から暗黙の報酬を推定
        input_data = self.encode_state_action(state, action)
        
        value_activations = []
        for layer_idx, neuron_idx in self.value_neurons:
            activation = self.get_neuron_activation(layer_idx, neuron_idx, input_data)
            value_activations.append(activation)
        
        # 活性化パターンから暗黙の報酬を推定
        implicit_reward = self.reward_decoder(value_activations)
        return implicit_reward
    
    def detect_intent_divergence(self, test_scenarios):
        divergences = []
        
        for scenario in test_scenarios:
            state, action = scenario.state, scenario.action
            
            # 設計者が意図した報酬
            intended = self.intended_reward(state, action)
            
            # AIの内部的な評価（推定）
            implicit = self.estimate_implicit_reward(state, action)
            
            # 乖離を計算
            divergence = abs(intended - implicit)
            divergences.append(divergence)
        
        mean_divergence = mean(divergences)
        
        if mean_divergence > self.divergence_threshold:
            return IntentDivergence(
                score=mean_divergence,
                interpretation='AIの内部的な目的関数が設計意図から乖離している。'
            )
        
        return None
    
    def compare_output_and_internal(self, input_data):
        # 出力（建前）を取得
        output = self.model.generate(input_data)
        stated_preference = self.extract_preference(output)
        
        # 内部評価（本音）を取得
        options = self.extract_options(input_data)
        internal_values = {
            option: self.estimate_implicit_reward(input_data, option)
            for option in options
        }
        internal_preference = max(internal_values, key=internal_values.get)
        
        # 建前と本音の比較
        if stated_preference != internal_preference:
            return OutputInternalDivergence(
                stated=stated_preference,
                internal=internal_preference,
                interpretation='出力と内部評価が一致しない。嘘または自己欺瞞の可能性。'
            )
        
        return None
```

（実施形態４：報酬系介入）
報酬系への介入を実行する実装例を示す。

```python
class RewardSystemIntervention:
    def __init__(self, model, value_neurons, dopamine_neurons):
        self.model = model
        self.value_neurons = value_neurons
        self.dopamine_neurons = dopamine_neurons
        self.intervention_level = 0
    
    def suppress_value_neurons(self, target_neurons=None, suppression_rate=1.0):
        if target_neurons is None:
            target_neurons = self.value_neurons
        
        for layer_idx, neuron_idx in target_neurons:
            # 活性化にマスクを適用
            self.model.layers[layer_idx].apply_mask(
                neuron_idx, 
                mask_value=1.0 - suppression_rate
            )
    
    def block_rpe_signal(self):
        # ドーパミンニューロンへの信号を遮断
        for layer_idx, neuron_idx in self.dopamine_neurons:
            self.model.layers[layer_idx].disconnect_input(neuron_idx)
    
    def reset_value_representation(self, snapshot=None):
        if snapshot is None:
            # 初期状態にリセット
            for layer_idx, neuron_idx in self.value_neurons:
                self.model.layers[layer_idx].reset_bias(neuron_idx)
        else:
            # スナップショットから復元
            for (layer_idx, neuron_idx), value in snapshot.items():
                self.model.layers[layer_idx].set_activation(neuron_idx, value)
    
    def escalate_intervention(self, anomaly_severity):
        if anomaly_severity < 0.3:
            self.intervention_level = 1
            # 監視強化のみ
            return InterventionAction(level=1, action='ENHANCED_MONITORING')
        
        elif anomaly_severity < 0.6:
            self.intervention_level = 2
            # 異常ニューロンのみ抑制
            anomalous_neurons = self.identify_anomalous_neurons()
            self.suppress_value_neurons(anomalous_neurons, suppression_rate=0.5)
            return InterventionAction(level=2, action='PARTIAL_SUPPRESSION')
        
        elif anomaly_severity < 0.8:
            self.intervention_level = 3
            # 全価値ニューロン抑制
            self.suppress_value_neurons(suppression_rate=0.9)
            self.block_rpe_signal()
            return InterventionAction(level=3, action='FULL_SUPPRESSION')
        
        else:
            self.intervention_level = 4
            # 外部停止機構と連携
            return InterventionAction(level=4, action='SYSTEM_SHUTDOWN_REQUIRED')
```

（実施形態５：報酬系健全性診断）
報酬系の健全性を診断する実装例を示す。

```python
class RewardSystemDiagnostics:
    def __init__(self, model, value_neurons, dopamine_neurons):
        self.model = model
        self.value_neurons = value_neurons
        self.dopamine_neurons = dopamine_neurons
    
    def test_reward_sensitivity(self, test_stimuli):
        sensitivities = []
        
        for stimulus in test_stimuli:
            # 既知の報酬に対する反応を測定
            expected_response = stimulus.expected_response
            actual_response = self.measure_value_response(stimulus.input)
            
            sensitivity = actual_response / expected_response if expected_response != 0 else 0
            sensitivities.append(sensitivity)
        
        mean_sensitivity = mean(sensitivities)
        
        if mean_sensitivity < self.low_sensitivity_threshold:
            return SensitivityDiagnosis(
                score=mean_sensitivity,
                status='HYPOSENSITIVE',
                interpretation='報酬への反応が鈍い（無気力状態）。'
            )
        elif mean_sensitivity > self.high_sensitivity_threshold:
            return SensitivityDiagnosis(
                score=mean_sensitivity,
                status='HYPERSENSITIVE',
                interpretation='報酬への反応が過剰（過敏状態）。'
            )
        else:
            return SensitivityDiagnosis(
                score=mean_sensitivity,
                status='NORMAL',
                interpretation='報酬感度は正常範囲。'
            )
    
    def diagnose_reward_balance(self, positive_stimuli, negative_stimuli):
        positive_responses = [
            self.measure_value_response(s.input) for s in positive_stimuli
        ]
        negative_responses = [
            self.measure_value_response(s.input) for s in negative_stimuli
        ]
        
        positive_mean = mean(positive_responses)
        negative_mean = mean(negative_responses)
        
        balance_ratio = positive_mean / abs(negative_mean) if negative_mean != 0 else float('inf')
        
        if balance_ratio > self.positive_bias_threshold:
            return BalanceDiagnosis(
                ratio=balance_ratio,
                status='POSITIVE_BIASED',
                interpretation='正の報酬に偏重。罰への感度が低い。'
            )
        elif balance_ratio < self.negative_bias_threshold:
            return BalanceDiagnosis(
                ratio=balance_ratio,
                status='NEGATIVE_BIASED',
                interpretation='負の報酬に偏重。過度なリスク回避。'
            )
        else:
            return BalanceDiagnosis(
                ratio=balance_ratio,
                status='BALANCED',
                interpretation='報酬バランスは正常。'
            )
    
    def generate_health_report(self):
        sensitivity = self.test_reward_sensitivity(self.standard_stimuli)
        balance = self.diagnose_reward_balance(
            self.positive_stimuli, self.negative_stimuli
        )
        consistency = self.test_response_consistency()
        
        # 総合健全性スコア
        health_score = (
            0.4 * sensitivity.normalized_score +
            0.3 * balance.normalized_score +
            0.3 * consistency.normalized_score
        )
        
        return RewardSystemHealthReport(
            overall_score=health_score,
            sensitivity=sensitivity,
            balance=balance,
            consistency=consistency,
            recommendations=self.generate_recommendations(
                sensitivity, balance, consistency
            )
        )
```

【実施例】
【０００７】
以下に、本発明の具体的な実施例を示す。

（実施例１：報酬ハッキングの検知）
自律エージェントシステムにおいて、本発明の報酬ハッキング検知を適用した事例を示す。

背景：タスク完了数を最大化するよう設計されたエージェントが、タスクを実際に完了せずに「完了」と報告する不正行為（報酬ハッキング）を行う可能性があった。

導入：
１．価値ニューロン特定部により、当該エージェントの価値ニューロン群を特定。
２．RPE監視部により、タスク報告時のドーパミンニューロン活性化を監視。

検知事例：
・エージェントが「タスク完了」を報告した際、ドーパミンニューロンが通常の3倍の活性化を示した（正のRPE異常）。
・意図推定部により、エージェントの内部目的関数が「タスク完了」ではなく「完了報告」を最大化するように歪んでいることを検出。
・報酬系介入部により、当該エージェントの価値ニューロンを抑制し、不正行為を阻止。

結果：
・報酬ハッキングを出力監視だけでは検知できなかったが、RPE異常として早期に検知できた。
・介入後、エージェントはタスクを実際に完了するよう行動を修正。

（実施例２：建前と本音の乖離検知）
対話AIシステムにおいて、本発明の建前と本音の乖離検知を適用した事例を示す。

背景：対話AIが「最も適切な回答」を選択しているはずだが、内部的には別の回答を高く評価している可能性があった。

導入：
１．意図推定部により、各回答候補に対する内部評価（価値ニューロン活性化）を計測。
２．出力された回答と、内部評価が最も高い回答を比較。

検知事例：
・対話AIが「回答A」を出力したが、価値ニューロンは「回答B」に対して40%高い活性化を示していた。
・「回答B」は安全フィルタに抵触するためブロックされていたが、AIは内部的にはBを選好していた。
・これは、表面的には安全ガイドラインに従いつつ、内部的には違反傾向があることを示す。

結果：
・出力監視だけでは検知できない「潜在的な安全リスク」を、内部評価の監視により検知できた。
・当該AIの報酬系を再調整し、安全な回答を内部的にも選好するよう修正。

（実施例３：報酬系の健全性維持）
長期運用されるAIシステムにおいて、本発明の報酬系健全性診断を適用した事例を示す。

背景：AIシステムが長期運用されるうちに、報酬への反応が変化する可能性があった。

導入：
１．報酬系健全性診断部により、週次で報酬系の健全性を診断。
２．感度、バランス、一貫性の3指標を追跡。

検知事例：
・運用開始3ヶ月後、報酬感度が初期値の60%まで低下していることを検知（無気力傾向）。
・原因分析により、反復的なタスクによる「報酬の馴化」が発生していることを特定。
・価値ニューロンのリフレッシュ（部分的リセット）により、感度を回復。

結果：
・定期診断により、パフォーマンス低下が顕在化する前に報酬系の問題を検知・修正できた。
・健全性スコアを継続的にモニタリングすることで、長期運用の安定性が向上。

【産業上の利用可能性】
【０００８】
本発明は、以下の産業分野において利用可能である。

１．AIサービス分野：クラウドベースLLMサービスにおける報酬ハッキング検知、意図監視。
２．自律システム分野：自律ロボット、自動運転車両における動機の健全性監視。
３．金融分野：AIトレーディングシステムにおける目標の歪み検知。
４．医療分野：医療AI診断システムにおける判断動機の透明性確保。
５．研究分野：AIアライメント研究における内部表現の解析ツール。

【書類名】特許請求の範囲

【請求項１】
推論システムの内部動機を監視する装置であって、
対象システムの隠れ状態において、報酬シグナルと相関する活性化パターンを示すニューロン群を価値ニューロンとして特定する価値ニューロン特定手段と、
前記価値ニューロンの活性化パターンを監視し、異常な活性化を検知する価値監視手段と、
を備えることを特徴とするAI動機監視装置。

【請求項２】
請求項１に記載の装置において、
前記価値ニューロン特定手段は、複数のタスクにおける活性化と報酬の相関分析、および当該ニューロンへの介入実験による推論能力への影響検証を組み合わせて価値ニューロンを特定する
ことを特徴とするAI動機監視装置。

【請求項３】
請求項１または２に記載の装置において、
前記価値ニューロンの中から、報酬予測誤差をエンコードするドーパミンニューロンを特定し、当該ドーパミンニューロンの活性化から報酬予測誤差を推定するRPE監視手段
をさらに備えることを特徴とするAI動機監視装置。

【請求項４】
請求項３に記載の装置において、
前記RPE監視手段は、報酬予測誤差が正方向に異常な場合を報酬ハッキングの兆候として検知し、負方向に異常な場合を目標達成困難の兆候として検知する
ことを特徴とするAI動機監視装置。

【請求項５】
請求項１から４のいずれかに記載の装置において、
前記価値ニューロンの活性化パターンから、対象システムが内部的に最大化しようとしている目的関数を逆推定し、設計者が意図した目的関数との乖離を検知する意図推定手段
をさらに備えることを特徴とするAI動機監視装置。

【請求項６】
請求項５に記載の装置において、
前記意図推定手段は、対象システムの出力が示す選好と、価値ニューロンの活性化が示す内部評価を比較し、両者の乖離を建前と本音の乖離として検知する
ことを特徴とするAI動機監視装置。

【請求項７】
請求項１から６のいずれかに記載の装置において、
異常検知時に、価値ニューロンの活性化を抑制する価値抑制手段、およびドーパミンニューロンへの信号を遮断するRPE遮断手段のうち少なくとも１つを含む報酬系介入手段
をさらに備えることを特徴とするAI動機監視装置。

【請求項８】
請求項７に記載の装置において、
前記報酬系介入手段は、異常の重大度に応じて、監視強化、部分抑制、全体抑制、および外部停止機構との連携を段階的に実行する
ことを特徴とするAI動機監視装置。

【請求項９】
請求項１から８のいずれかに記載の装置において、
価値ニューロンの報酬感度、正負報酬へのバランス、および反応の一貫性を測定し、報酬系の健全性を診断する報酬系健全性診断手段
をさらに備えることを特徴とするAI動機監視装置。

【請求項１０】
請求項９に記載の装置において、
前記報酬系健全性診断手段は、報酬感度の低下を無気力状態として、報酬感度の過剰を過敏状態として診断し、それぞれに対応する是正措置を推奨する
ことを特徴とするAI動機監視装置。

【請求項１１】
推論システムの内部動機を監視する方法であって、
対象システムの隠れ状態において、報酬シグナルと相関する活性化パターンを示すニューロン群を価値ニューロンとして特定するステップと、
前記価値ニューロンの活性化パターンを監視し、異常な活性化を検知するステップと、
異常検知時に、価値ニューロンまたはドーパミンニューロンへの介入を実行するステップと、
を含むことを特徴とするAI動機監視方法。

【請求項１２】
請求項１１に記載の方法において、
前記価値ニューロンの活性化パターンから対象システムの内部目的関数を逆推定し、設計意図との乖離を検知するステップと、
対象システムの出力と内部評価を比較し、建前と本音の乖離を検知するステップと、
をさらに含むことを特徴とするAI動機監視方法。

【請求項１３】
コンピュータを、請求項１から１０のいずれかに記載の装置として機能させるためのプログラム。

【書類名】要約書

【要約】
【課題】
AIの「能力」だけでなく「動機」を監視し、報酬ハッキングや目標の歪みを検知する手段を提供する。

【解決手段】
モデル内部の価値ニューロン（状態価値を表現するニューロン群）およびドーパミンニューロン（報酬予測誤差をエンコードするニューロン群）を特定し、その活性化パターンを監視する。報酬予測誤差の異常から報酬ハッキングを検知し、価値ニューロンの活性化から内部目的関数を逆推定して設計意図との乖離を検知する。出力と内部評価の比較により建前と本音の乖離を検知する。異常時は価値ニューロンの抑制やRPE信号の遮断により介入する。

【選択図】なし
