import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import Logos.Theme
import Logos.Controls

Rectangle {
    id: ui

    color: Theme.palette.background
    readonly property int addressFieldWidth: 440
    readonly property int paramFieldWidth: 280

    // Temporary layout switch. Delete with D21.14 when chain writes land.
    property bool demoMode: true
    readonly property string demoOwnerId: "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU"
    readonly property string demoProviderId: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"
    property string stage: "needVault"
    property bool vaultExists: false
    property bool vaultHasHolding: false
    property bool streamExists: false
    property int streamStateCode: -1
    property string pendingWrite: ""
    property string lastError: "—"
    property string snapshotWalletBalance: "—"
    property string snapshotVaultHolding: "—"
    property string snapshotTotalAllocated: "—"
    property string snapshotRate: "—"
    property string snapshotAllocation: "—"
    property string snapshotAccrued: "—"
    property string snapshotUnaccrued: "—"
    property string snapshotAccrualStarted: "—"
    property string snapshotChainTime: "—"
    property string snapshotDepletedAt: "—"

    readonly property bool writeBusy: pendingWrite.length > 0
    readonly property int demoConfirmMs: 2000
    readonly property int localnetBlockTimeMs: 15000
    readonly property int confirmEtaSeconds: demoMode
        ? Math.round(demoConfirmMs / 1000)
        : Math.round(localnetBlockTimeMs / 1000)
    readonly property string confirmingLabel: "Confirming… ~" + confirmEtaSeconds + "s"
    property string pendingNextStage: ""
    readonly property string ownerError: accountIdError(ownerField.value, true)
    readonly property string vaultIdError: u64Error(vaultIdField.value, "Enter a vault id")
    readonly property string streamIdError: u64Error(streamIdField.value, "Enter a stream id")
    readonly property string providerError: {
        var t = trimmed(providerField.value)
        var required = stage === "needStream" || stage === "needClaim"
        var fmt = accountIdError(t, required)
        if (fmt.length > 0)
            return fmt
        if (t.length > 0 && accountsEqual(ownerField.value, t) && required)
            return "Provider must differ from owner"
        return ""
    }
    readonly property string amountError: positiveU64Error(
                                              depositAmountField.value,
                                              "Enter an amount greater than 0",
                                              "Amount must be greater than 0")
    readonly property string rateError: positiveU64Error(
                                            rateField.value,
                                            "Enter a rate greater than 0",
                                            "Rate must be greater than 0")
    readonly property string allocationError: positiveU64Error(
                                                  allocationField.value,
                                                  "Enter an allocation greater than 0",
                                                  "Allocation must be greater than 0")
    readonly property bool providerCloseOk: {
        var t = trimmed(providerField.value)
        return accountIdError(t, true).length === 0 && !accountsEqual(ownerField.value, t)
    }
    readonly property bool canInitialize: stage === "needVault" && !writeBusy
        && ownerError.length === 0 && vaultIdError.length === 0
    readonly property bool canDeposit: stage === "needDeposit" && !writeBusy
        && ownerError.length === 0 && vaultIdError.length === 0 && amountError.length === 0
    readonly property bool canCreateStream: stage === "needStream" && !writeBusy
        && ownerError.length === 0 && vaultIdError.length === 0
        && streamIdError.length === 0 && providerError.length === 0
        && rateError.length === 0 && allocationError.length === 0
    readonly property bool canClose: stage === "needClose" && !writeBusy
        && ownerError.length === 0 && vaultIdError.length === 0 && streamIdError.length === 0
    readonly property bool canClaim: stage === "needClaim" && !writeBusy
        && ownerError.length === 0 && vaultIdError.length === 0
        && streamIdError.length === 0 && providerError.length === 0
    readonly property string streamBadgeText: {
        if (!vaultExists)
            return "No vault"
        if (!streamExists)
            return "No stream"
        if (streamStateCode === 0)
            return "Active"
        if (streamStateCode === 1)
            return "Paused"
        if (streamStateCode === 2)
            return "Closed"
        return "Unknown"
    }
    readonly property string nextActionLabel: {
        if (writeBusy)
            return confirmingLabel
        if (stage === "needVault")
            return "Initialize vault"
        if (stage === "needDeposit")
            return "Deposit"
        if (stage === "needStream")
            return "Create stream"
        if (stage === "needClose")
            return "Close stream"
        if (stage === "needClaim")
            return "Claim"
        return "Complete"
    }

    function trimmed(s) {
        return String(s === undefined || s === null ? "" : s).trim()
    }

    function isHex64(s) {
        return /^[0-9a-fA-F]{64}$/.test(s)
    }

    function isBase58Account(s) {
        return s.length >= 32 && s.length <= 44 && /^[1-9A-HJ-NP-Za-km-z]+$/.test(s)
    }

    function accountIdError(s, required) {
        var t = trimmed(s)
        if (t.length === 0)
            return required ? "Enter an account id" : ""
        if (isHex64(t) || isBase58Account(t))
            return ""
        return "Use base58 or 64-hex"
    }

    function u64Error(s, emptyMsg) {
        var t = trimmed(s)
        if (t.length === 0)
            return emptyMsg
        if (!/^(0|[1-9][0-9]*)$/.test(t))
            return "Use a whole number 0 or greater"
        if (t.length > 20 || (t.length === 20 && t > "18446744073709551615"))
            return "Must fit in 64 bits"
        return ""
    }

    function positiveU64Error(s, emptyMsg, zeroMsg) {
        var e = u64Error(s, emptyMsg)
        if (e.length > 0)
            return e
        if (trimmed(s) === "0")
            return zeroMsg
        return ""
    }

    function normalizeAccount(s) {
        var t = trimmed(s)
        if (isHex64(t))
            return t.toLowerCase()
        return t
    }

    function accountsEqual(a, b) {
        var left = normalizeAccount(a)
        var right = normalizeAccount(b)
        return left.length > 0 && left === right
    }

    function copyToClipboard(s) {
        clipboardHelper.text = s
        clipboardHelper.selectAll()
        clipboardHelper.copy()
    }

    function parseCall(raw) {
        var v = raw
        if (typeof v === "string") {
            var trimmed = v.trim()
            if (trimmed.length === 0)
                return undefined
            try {
                v = JSON.parse(trimmed)
            } catch (e) {
                return undefined
            }
        }
        if (v && typeof v === "object" && v.success === false)
            return undefined
        if (v && typeof v === "object" && ("success" in v) && ("data" in v))
            v = v.data
        if (typeof v === "string") {
            try {
                v = JSON.parse(v)
            } catch (e) {
            }
        }
        return v
    }

    function callJson(moduleName, methodName, args) {
        if (typeof logos === "undefined" || !logos.callModule)
            return undefined
        try {
            return parseCall(logos.callModule(moduleName, methodName, args))
        } catch (e) {
            return undefined
        }
    }

    function accountToBase58(accountIdHex) {
        var b58 = callJson("logos_execution_zone", "account_id_to_base58", [accountIdHex])
        if (typeof b58 === "string" && b58.length > 0)
            return b58
        return ""
    }

    function publicAccountIds() {
        var listed = callJson("logos_execution_zone", "list_accounts", [])
        var out = []
        if (!listed || typeof listed.length !== "number")
            return out
        for (var i = 0; i < listed.length; ++i) {
            var entry = listed[i]
            if (!entry || entry.is_public === false)
                continue
            var hex = entry.account_id || entry.accountId || ""
            if (typeof hex !== "string" || hex.length === 0)
                continue
            var b58 = accountToBase58(hex)
            if (b58.length > 0)
                out.push(b58)
        }
        return out
    }

    function syncWalletMirror() {
        var height = Number(callJson("logos_execution_zone", "get_current_block_height", []))
        if (height > 0)
            callJson("logos_execution_zone", "sync_to_block", [height])
    }

    function vaultStatus(owner, vaultId) {
        var params = JSON.stringify({
            "owner": owner,
            "vault_id": vaultId
        })
        var status = callJson("payment_streams_module", "chainAction", ["getVaultStatus", params])
        if (!status || status.status !== "ok")
            return undefined
        return status
    }

    function streamStatus(owner, vaultId, streamId) {
        var params = JSON.stringify({
            "owner": owner,
            "vault_id": vaultId,
            "stream_id": streamId
        })
        var status = callJson("payment_streams_module", "chainAction", ["getStreamStatus", params])
        if (!status || status.status !== "ok")
            return undefined
        return status
    }

    function holdingIsPositive(hex) {
        if (typeof hex !== "string" || hex.length === 0)
            return false
        return /[1-9a-f]/i.test(hex)
    }

    function clearSnapshot() {
        snapshotWalletBalance = "—"
        snapshotVaultHolding = "—"
        snapshotTotalAllocated = "—"
        snapshotRate = "—"
        snapshotAllocation = "—"
        snapshotAccrued = "—"
        snapshotUnaccrued = "—"
        snapshotAccrualStarted = "—"
        snapshotChainTime = "—"
        snapshotDepletedAt = "—"
    }

    function setStage(next, streamCode) {
        stage = next
        if (next === "needVault") {
            vaultExists = false
            vaultHasHolding = false
            streamExists = false
            streamStateCode = -1
        } else if (next === "needDeposit") {
            vaultExists = true
            vaultHasHolding = false
            streamExists = false
            streamStateCode = -1
        } else if (next === "needStream") {
            vaultExists = true
            vaultHasHolding = true
            streamExists = false
            streamStateCode = -1
        } else if (next === "needClose") {
            vaultExists = true
            vaultHasHolding = true
            streamExists = true
            streamStateCode = (streamCode === 1) ? 1 : 0
        } else {
            vaultExists = true
            vaultHasHolding = true
            streamExists = true
            streamStateCode = 2
        }
    }

    function applyDemoSnapshot() {
        if (stage === "needVault") {
            clearSnapshot()
            return
        }
        snapshotWalletBalance = "2 000"
        snapshotAccrualStarted = "—"
        snapshotChainTime = "—"
        snapshotDepletedAt = "—"
        if (stage === "needDeposit") {
            snapshotVaultHolding = "0"
            snapshotTotalAllocated = "0"
            snapshotRate = "—"
            snapshotAllocation = "—"
            snapshotAccrued = "—"
            snapshotUnaccrued = "—"
            return
        }
        if (stage === "needStream") {
            snapshotWalletBalance = "1 500"
            snapshotVaultHolding = "500"
            snapshotTotalAllocated = "0"
            snapshotRate = "—"
            snapshotAllocation = "—"
            snapshotAccrued = "—"
            snapshotUnaccrued = "—"
            return
        }
        snapshotWalletBalance = "1 500"
        snapshotRate = "1"
        snapshotAllocation = "80"
        snapshotAccrualStarted = "2026-08-17 07:12:04 UTC"
        snapshotChainTime = "2026-08-17 07:32:44 UTC"
        if (stage === "needClose") {
            snapshotVaultHolding = "420"
            snapshotTotalAllocated = "80"
            snapshotAccrued = "12"
            snapshotUnaccrued = "68"
            snapshotDepletedAt = "2026-08-17 08:40:44 UTC"
            return
        }
        snapshotUnaccrued = "0"
        snapshotDepletedAt = "—"
        if (stage === "needClaim") {
            snapshotVaultHolding = "488"
            snapshotTotalAllocated = "12"
            snapshotAccrued = "12"
            return
        }
        snapshotVaultHolding = "488"
        snapshotTotalAllocated = "0"
        snapshotAccrued = "0"
    }

    function actionOpen(stageName, action) {
        if (stage !== stageName)
            return false
        return pendingWrite.length === 0 || pendingWrite === action
    }

    function nextStageForAction(name) {
        if (name === "initializeVault")
            return "needDeposit"
        if (name === "deposit")
            return "needStream"
        if (name === "createStream")
            return "needClose"
        if (name === "ownerClose" || name === "providerClose")
            return "needClaim"
        if (name === "claim")
            return "done"
        return ""
    }

    function actionAllowed(name) {
        if (name === "initializeVault")
            return canInitialize
        if (name === "deposit")
            return canDeposit
        if (name === "createStream")
            return canCreateStream
        if (name === "ownerClose")
            return canClose
        if (name === "providerClose")
            return canClose && providerCloseOk
        if (name === "claim")
            return canClaim
        return false
    }

    function runAction(name) {
        if (writeBusy || !actionAllowed(name))
            return
        lastError = "—"
        if (!ui.demoMode) {
            lastError = "Turn on Demo mode to walk these actions on this screen"
            return
        }
        pendingWrite = name
        pendingNextStage = nextStageForAction(name)
        demoConfirmTimer.restart()
    }

    function finishDemoAction() {
        var next = pendingNextStage
        pendingWrite = ""
        pendingNextStage = ""
        if (next.length === 0)
            return
        setStage(next)
        applyDemoSnapshot()
    }

    function setDemoMode(on) {
        demoMode = on
        lastError = "—"
        pendingWrite = ""
        pendingNextStage = ""
        demoConfirmTimer.stop()
        if (on) {
            fillDemoAccounts()
            setStage("needVault")
            applyDemoSnapshot()
            return
        }
        clearSnapshot()
        setStage("needVault")
        loadSessionDefaults()
    }

    function applyVaultSnapshot(found, owner) {
        vaultIdField.value = String(found.vault_id)
        var holding = holdingIsPositive(found.vault_holding_balance_hex || "")
        var nextStream = 0
        if (found.vault_config && found.vault_config.next_stream_id !== undefined)
            nextStream = Number(found.vault_config.next_stream_id)
        if (nextStream <= 0) {
            setStage(holding ? "needStream" : "needDeposit")
            return
        }

        streamIdField.value = String(nextStream - 1)
        var st = streamStatus(owner, found.vault_id, nextStream - 1)
        if (!st) {
            setStage(holding ? "needStream" : "needDeposit")
            return
        }
        var code = Number(st.stream_state)
        if (code === 0 || code === 1)
            setStage("needClose", code)
        else if (code === 2)
            setStage("needClaim")
        else
            setStage("done")
    }

    function fillDemoAccounts() {
        var accounts = publicAccountIds()
        if (accounts.length > 0)
            ownerField.value = accounts[0]
        else
            ownerField.value = demoOwnerId
        if (accounts.length > 1)
            providerField.value = accounts[1]
        else
            providerField.value = demoProviderId
    }

    function loadSessionDefaults() {
        if (demoMode) {
            fillDemoAccounts()
            setStage("needVault")
            applyDemoSnapshot()
            return
        }

        var accounts = publicAccountIds()
        if (accounts.length > 0)
            ownerField.value = accounts[0]
        if (accounts.length > 1)
            providerField.value = accounts[1]

        var owner = ownerField.value.trim()
        if (owner.length === 0)
            return

        syncWalletMirror()

        var found = undefined
        var probeIds = [0, 1, 2]
        for (var i = 0; i < probeIds.length; ++i) {
            var st = vaultStatus(owner, probeIds[i])
            if (st) {
                found = st
                break
            }
        }
        if (!found) {
            setStage("needVault")
            return
        }

        applyVaultSnapshot(found, owner)
    }

    Component.onCompleted: Qt.callLater(ui.loadSessionDefaults)

    TextEdit {
        id: clipboardHelper
        width: 1
        height: 1
        opacity: 0
    }

    Timer {
        id: demoConfirmTimer
        interval: ui.demoConfirmMs
        repeat: false
        onTriggered: ui.finishDemoAction()
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: pageCol.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ColumnLayout {
            id: pageCol
            width: ui.width
            spacing: Theme.spacing.medium

            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing.medium
                Layout.rightMargin: Theme.spacing.medium
                Layout.topMargin: Theme.spacing.medium

                LogosText {
                    text: "payment_streams_ui"
                    font.pixelSize: Theme.typography.panelTitleText
                    font.weight: Theme.typography.weightBold
                    color: Theme.palette.text
                }

                Item {
                    Layout.fillWidth: true
                }

                LogosText {
                    text: "Demo mode"
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }

                LogosSwitch {
                    checked: ui.demoMode
                    onToggled: ui.setDemoMode(checked)
                }
            }

            SectionCard {
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing.medium
                Layout.rightMargin: Theme.spacing.medium
                title: "Session"

                LogosText {
                    Layout.fillWidth: true
                    text: ui.demoMode
                          ? "Demo mode walks these actions on this screen."
                          : "These are the wallet defaults. Continue with them, or paste other account ids from the LEZ wallet UI."
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                    wrapMode: Text.Wrap
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.medium

                    IdField {
                        id: ownerField
                        Layout.fillWidth: true
                        label: "owner"
                        value: ""
                        errorText: ui.ownerError
                    }
                    IdField {
                        id: vaultIdField
                        Layout.fillWidth: true
                        label: "vault_id"
                        value: "0"
                        errorText: ui.vaultIdError
                    }
                    IdField {
                        id: providerField
                        Layout.fillWidth: true
                        label: "provider"
                        value: ""
                        errorText: ui.providerError
                    }
                    IdField {
                        id: streamIdField
                        Layout.fillWidth: true
                        label: "stream_id"
                        value: "0"
                        errorText: ui.streamIdError
                    }
                }

                LogosText {
                    Layout.fillWidth: true
                    text: "Last error: " + ui.lastError
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                    wrapMode: Text.Wrap
                }

                LogosText {
                    Layout.fillWidth: true
                    text: "Pending write: " + (ui.pendingWrite.length > 0 ? ui.pendingWrite : "none")
                    color: Theme.palette.textTertiary
                    font.pixelSize: Theme.typography.secondaryText
                }
            }

            SectionCard {
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing.medium
                Layout.rightMargin: Theme.spacing.medium
                title: "On-chain state"

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.medium

                    LogosText {
                        Layout.fillWidth: true
                        text: ui.demoMode
                              ? "Demo snapshot of vault holding, stream fold, and owner wallet balance."
                              : "Read-only snapshot of vault holding, stream fold, and owner wallet balance. Refresh re-reads those from chain."
                        color: Theme.palette.textSecondary
                        font.pixelSize: Theme.typography.secondaryText
                        wrapMode: Text.Wrap
                    }

                    LogosButton {
                        text: "Refresh"
                        Layout.preferredHeight: 40
                        Layout.alignment: Qt.AlignTop
                        onClicked: {
                            if (ui.demoMode)
                                ui.applyDemoSnapshot()
                            else
                                ui.loadSessionDefaults()
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.medium

                    LogosText {
                        text: "Stream"
                        font.pixelSize: Theme.typography.primaryText
                        font.weight: Theme.typography.weightBold
                        color: Theme.palette.text
                    }

                    LogosBadge {
                        text: ui.streamBadgeText
                        color: ui.streamStateCode === 0 ? Theme.palette.success : Theme.palette.textSecondary
                    }

                    LogosText {
                        text: "Next: " + ui.nextActionLabel
                        color: Theme.palette.textSecondary
                        font.pixelSize: Theme.typography.secondaryText
                    }

                    Item {
                        Layout.fillWidth: true
                    }
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: 2
                    columnSpacing: Theme.spacing.medium
                    rowSpacing: Theme.spacing.small

                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Owner wallet balance"
                        value: ui.snapshotWalletBalance
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Vault holding"
                        value: ui.snapshotVaultHolding
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Total allocated"
                        value: ui.snapshotTotalAllocated
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Rate (tokens / s)"
                        value: ui.snapshotRate
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Allocation"
                        value: ui.snapshotAllocation
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Accrued"
                        value: ui.snapshotAccrued
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Unaccrued"
                        value: ui.snapshotUnaccrued
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Accrual started"
                        value: ui.snapshotAccrualStarted
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        label: "Chain time"
                        value: ui.snapshotChainTime
                    }
                    SnapshotValue {
                        Layout.fillWidth: true
                        Layout.columnSpan: 2
                        label: "Estimated depleted at"
                        value: ui.snapshotDepletedAt
                    }
                }

                LogosText {
                    Layout.fillWidth: true
                    text: "Snapshot as of last Refresh"
                    color: Theme.palette.textTertiary
                    font.pixelSize: Theme.typography.secondaryText
                }
            }

            SectionCard {
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing.medium
                Layout.rightMargin: Theme.spacing.medium
                title: "Owner"

                LogosText {
                    Layout.fillWidth: true
                    text: "Writes signed as owner."
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.medium

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needVault", "initializeVault")
                        title: "Initialize vault"
                        summary: "Creates an empty native vault for this owner and vault id."

                        ActionButton {
                            idleText: "Initialize vault"
                            confirming: ui.pendingWrite === "initializeVault"
                            actionEnabled: ui.canInitialize || confirming
                            onClicked: {
                                if (ui.canInitialize)
                                    ui.runAction("initializeVault")
                            }
                        }
                    }

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needDeposit", "deposit")
                        title: "Deposit"
                        summary: "Moves native tokens from the owner wallet into the vault holding."

                        ParamField {
                            id: depositAmountField
                            placeholderText: "amount"
                            value: "500"
                            enabled: ui.stage === "needDeposit" && !ui.writeBusy
                            errorText: ui.stage === "needDeposit" ? ui.amountError : ""
                        }

                        ActionButton {
                            idleText: "Deposit"
                            confirming: ui.pendingWrite === "deposit"
                            actionEnabled: ui.canDeposit || confirming
                            onClicked: {
                                if (ui.canDeposit)
                                    ui.runAction("deposit")
                            }
                        }
                    }

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needStream", "createStream")
                        title: "Create stream"
                        summary: "Opens a stream to the provider at a fixed rate, drawing from the vault allocation."

                        ParamField {
                            id: rateField
                            placeholderText: "rate (tokens / s)"
                            value: "1"
                            enabled: ui.stage === "needStream" && !ui.writeBusy
                            errorText: ui.stage === "needStream" ? ui.rateError : ""
                        }

                        ParamField {
                            id: allocationField
                            placeholderText: "allocation"
                            value: "80"
                            enabled: ui.stage === "needStream" && !ui.writeBusy
                            errorText: ui.stage === "needStream" ? ui.allocationError : ""
                        }

                        ActionButton {
                            idleText: "Create stream"
                            confirming: ui.pendingWrite === "createStream"
                            actionEnabled: ui.canCreateStream || confirming
                            onClicked: {
                                if (ui.canCreateStream)
                                    ui.runAction("createStream")
                            }
                        }
                    }

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needClose", "ownerClose")
                        title: "Owner close"
                        summary: "Stops accrual on this stream and returns unaccrued tokens to the vault."

                        ActionButton {
                            idleText: "Owner close"
                            confirming: ui.pendingWrite === "ownerClose"
                            actionEnabled: ui.canClose || confirming
                            onClicked: {
                                if (ui.canClose)
                                    ui.runAction("ownerClose")
                            }
                        }
                    }
                }
            }

            SectionCard {
                Layout.fillWidth: true
                Layout.leftMargin: Theme.spacing.medium
                Layout.rightMargin: Theme.spacing.medium
                Layout.bottomMargin: Theme.spacing.medium
                title: "Provider"

                LogosText {
                    Layout.fillWidth: true
                    text: "Writes signed as provider."
                    color: Theme.palette.textSecondary
                    font.pixelSize: Theme.typography.secondaryText
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacing.medium

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needClaim", "claim")
                        title: "Claim"
                        summary: "Pays the provider the amount accrued on this stream as of the last Refresh."

                        ActionButton {
                            idleText: "Claim"
                            confirming: ui.pendingWrite === "claim"
                            actionEnabled: ui.canClaim || confirming
                            onClicked: {
                                if (ui.canClaim)
                                    ui.runAction("claim")
                            }
                        }
                    }

                    OperationBlock {
                        Layout.fillWidth: true
                        available: ui.actionOpen("needClose", "providerClose")
                        title: "Provider close"
                        summary: "Stops this stream as the provider and returns unaccrued tokens to the vault."

                        ActionButton {
                            idleText: "Provider close"
                            confirming: ui.pendingWrite === "providerClose"
                            actionEnabled: (ui.canClose && ui.providerCloseOk) || confirming
                            onClicked: {
                                if (ui.canClose && ui.providerCloseOk)
                                    ui.runAction("providerClose")
                            }
                        }
                    }
                }
            }
        }
    }

    component SectionCard: Rectangle {
        default property alias content: body.data
        property alias title: titleText.text

        color: Theme.palette.backgroundSecondary
        radius: Theme.spacing.radiusMedium
        border.width: 1
        border.color: Theme.palette.borderHairline
        implicitHeight: cardCol.implicitHeight + Theme.spacing.medium * 2

        ColumnLayout {
            id: cardCol
            anchors.fill: parent
            anchors.margins: Theme.spacing.medium
            spacing: Theme.spacing.small

            LogosText {
                id: titleText
                font.pixelSize: Theme.typography.primaryText
                font.weight: Theme.typography.weightBold
                color: Theme.palette.text
            }

            ColumnLayout {
                id: body
                Layout.fillWidth: true
                spacing: Theme.spacing.small
            }
        }
    }

    component ActionButton: LogosButton {
        property string idleText: ""
        property bool confirming: false
        property bool actionEnabled: false

        text: confirming ? ui.confirmingLabel : idleText
        enabled: actionEnabled
        Layout.preferredHeight: 40
    }

    component OperationBlock: Rectangle {
        default property alias content: body.data
        property alias title: titleText.text
        property alias summary: summaryText.text
        property bool available: true

        enabled: available
        opacity: available ? 1 : 0.45
        color: Theme.palette.background
        radius: Theme.spacing.radiusSmall
        border.width: 1
        border.color: Theme.palette.borderSubtle
        implicitHeight: blockCol.implicitHeight + Theme.spacing.medium * 2

        ColumnLayout {
            id: blockCol
            anchors.fill: parent
            anchors.margins: Theme.spacing.medium
            spacing: Theme.spacing.small

            LogosText {
                id: titleText
                font.pixelSize: Theme.typography.primaryText
                font.weight: Theme.typography.weightBold
                color: Theme.palette.text
            }

            LogosText {
                id: summaryText
                Layout.fillWidth: true
                wrapMode: Text.Wrap
                color: Theme.palette.textSecondary
                font.pixelSize: Theme.typography.secondaryText
            }

            ColumnLayout {
                id: body
                Layout.fillWidth: true
                spacing: Theme.spacing.small
            }
        }
    }

    component IdField: ColumnLayout {
        property alias label: lab.text
        property alias value: field.text
        property string errorText: ""
        spacing: Theme.spacing.tiny

        LogosText {
            id: lab
            color: Theme.palette.textSecondary
            font.pixelSize: Theme.typography.secondaryText
        }

        RowLayout {
            spacing: Theme.spacing.small

            LogosTextField {
                id: field
                Layout.preferredWidth: ui.addressFieldWidth
                Layout.maximumWidth: ui.addressFieldWidth
            }

            CopyButton {
                onClicked: ui.copyToClipboard(field.text)
            }
        }

        LogosText {
            visible: errorText.length > 0
            text: errorText
            color: Theme.palette.error
            font.pixelSize: Theme.typography.secondaryText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }

    component CopyButton: Item {
        id: copyBtn
        signal clicked
        property bool copied: false

        Layout.preferredWidth: copied ? 72 : 40
        Layout.preferredHeight: 40
        implicitWidth: copied ? 72 : 40
        implicitHeight: 40
        Accessible.role: Accessible.Button
        Accessible.name: copied ? "Copied" : "Copy"

        Rectangle {
            anchors.fill: parent
            radius: Theme.spacing.radiusSmall
            color: copyArea.containsMouse ? Theme.palette.backgroundButton : "transparent"
            border.width: 1
            border.color: Theme.palette.borderSubtle
        }

        CopyGlyph {
            anchors.centerIn: parent
            visible: !copyBtn.copied
        }

        LogosText {
            anchors.centerIn: parent
            visible: copyBtn.copied
            text: "Copied"
            color: Theme.palette.success
            font.pixelSize: Theme.typography.secondaryText
        }

        Timer {
            id: copiedReset
            interval: 1500
            onTriggered: copyBtn.copied = false
        }

        MouseArea {
            id: copyArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                copyBtn.clicked()
                copyBtn.copied = true
                copiedReset.restart()
            }
        }
    }

    component CopyGlyph: Item {
        implicitWidth: 16
        implicitHeight: 16

        Rectangle {
            x: 5
            y: 0
            width: 10
            height: 12
            radius: 1
            color: "transparent"
            border.width: 1.5
            border.color: Theme.palette.text
        }

        Rectangle {
            x: 1
            y: 4
            width: 10
            height: 12
            radius: 1
            color: Theme.palette.backgroundSecondary
            border.width: 1.5
            border.color: Theme.palette.text
        }
    }

    component ParamField: ColumnLayout {
        property alias placeholderText: field.placeholderText
        property alias value: field.text
        property alias enabled: field.enabled
        property string errorText: ""
        spacing: Theme.spacing.tiny

        LogosTextField {
            id: field
            Layout.preferredWidth: ui.paramFieldWidth
            Layout.maximumWidth: ui.paramFieldWidth
            validator: RegularExpressionValidator {
                regularExpression: /^(0|[1-9][0-9]{0,19})$/
            }
        }

        LogosText {
            visible: errorText.length > 0
            text: errorText
            color: Theme.palette.error
            font.pixelSize: Theme.typography.secondaryText
            wrapMode: Text.Wrap
            Layout.preferredWidth: ui.paramFieldWidth
        }
    }

    component SnapshotValue: ColumnLayout {
        property alias label: lab.text
        property alias value: val.text
        spacing: Theme.spacing.tiny

        LogosText {
            id: lab
            color: Theme.palette.textSecondary
            font.pixelSize: Theme.typography.secondaryText
        }

        LogosText {
            id: val
            color: Theme.palette.text
            font.pixelSize: Theme.typography.primaryText
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }
    }
}
