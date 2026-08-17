import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import Logos.Theme

Item {
    id: root

    ColumnLayout {
        anchors.centerIn: parent
        spacing: Theme.spacing.medium

        Text {
            Layout.alignment: Qt.AlignHCenter
            text: "payment_streams_ui"
            font.pixelSize: Theme.typography.panelTitleText
            font.weight: Theme.typography.weightBold
            color: Theme.palette.text
        }

        Button {
            Layout.alignment: Qt.AlignHCenter
            text: "test"
            onClicked: helloPopup.open()
        }
    }

    Popup {
        id: helloPopup
        parent: root
        x: Math.round((root.width - width) / 2)
        y: Math.round((root.height - height) / 2)
        modal: true
        focus: true
        padding: Theme.spacing.medium
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        Text {
            text: "Hello from payment_streams_ui"
            color: Theme.palette.text
            font.pixelSize: Theme.typography.primaryText
        }
    }
}
