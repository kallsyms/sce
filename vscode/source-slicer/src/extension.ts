import * as vscode from 'vscode';
import * as child_process from 'child_process';

const SLICER_BIN = "/tmp/slicer";

type SlicerRequest = {
    filename: string;
    content: string;
    point: [number, number];
};

type SlicerResponse = {
    content: string;
    point: [number, number];
};

const runSlicer = (request: SlicerRequest): Promise<SlicerResponse> => {
    return new Promise((resolve, reject) => {
        const proc = child_process.spawn(SLICER_BIN);
        proc.stdin.write(JSON.stringify(request));
        proc.stdin.end();
        proc.stdout.on('data', (data) => {
            const response = JSON.parse(data.toString());
            resolve(response);
        });
        proc.stderr.on('data', (data) => {
            reject(data.toString());
        });
    }
    );
};

export function activate(context: vscode.ExtensionContext) {
	let sliceCommand = vscode.commands.registerCommand('source-slicer.slice', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            return;
        }

        const filename = editor.document.fileName;
        const content = editor.document.getText();
        const language = editor.document.languageId;
        const point = editor.selection.active;

        const req: SlicerRequest = {
            filename,
            content,
            point: [point.line, point.character],
        };

        let resp: SlicerResponse;
        try {
            resp = await runSlicer(req);
        } catch (e) {
            vscode.window.showErrorMessage("Error slicing: ", e.toString());
            return;
        }

        const sliceDoc = await vscode.workspace.openTextDocument({
            language: language,
            content: resp.content,
        });
        // open the document as a preview
        vscode.window.showTextDocument(sliceDoc, {
            preview: true,
            selection: new vscode.Range(resp.point[0], resp.point[1], resp.point[0], resp.point[1]),
            viewColumn: vscode.ViewColumn.Beside,
        });
	});

	context.subscriptions.push(sliceCommand);
}

export function deactivate() {}
