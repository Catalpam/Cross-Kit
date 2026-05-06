import CrossKitFormWizardShared
import SwiftUI

struct ContentView: View {
    @StateObject private var kit = CrossKitFormWizardBridge()

    private var state: FormWizardState {
        kit.formWizard.state
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Form Wizard")
                .font(.title.bold())
                .accessibilityIdentifier("form.title")

            switch state.step {
            case .profile:
                ProfileStep(state: state, wizard: kit.formWizard)
            case .security:
                SecurityStep(state: state, wizard: kit.formWizard)
            case .summary:
                SummaryStep(state: state, wizard: kit.formWizard)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .padding()
    }
}

private struct ProfileStep: View {
    let state: FormWizardState
    let wizard: FormWizardViewModelBridge

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Profile")
                .font(.headline)
                .accessibilityIdentifier("form.step")
            TextField("Name", text: Binding(
                get: { state.name },
                set: { wizard.updateName(name: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("form.name")
            FieldError(text: state.nameError, id: "form.name.error")
            TextField("Email", text: Binding(
                get: { state.email },
                set: { wizard.updateEmail(email: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .textInputAutocapitalization(.never)
            .keyboardType(.emailAddress)
            .accessibilityIdentifier("form.email")
            FieldError(text: state.emailError, id: "form.email.error")
            Button("Next") { wizard.next() }
                .buttonStyle(.borderedProminent)
                .disabled(!state.canGoNext)
                .accessibilityIdentifier("form.next")
        }
    }
}

private struct SecurityStep: View {
    let state: FormWizardState
    let wizard: FormWizardViewModelBridge

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Security")
                .font(.headline)
                .accessibilityIdentifier("form.step")
            SecureField("Password", text: Binding(
                get: { state.password },
                set: { wizard.updatePassword(password: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("form.password")
            FieldError(text: state.passwordError, id: "form.password.error")
            SecureField("Confirm password", text: Binding(
                get: { state.confirmPassword },
                set: { wizard.updateConfirm(confirm: $0) }
            ))
            .textFieldStyle(.roundedBorder)
            .accessibilityIdentifier("form.confirm")
            FieldError(text: state.confirmPasswordError, id: "form.confirm.error")
            HStack(spacing: 12) {
                Button("Back") { wizard.back() }
                    .accessibilityIdentifier("form.back")
                Button("Next") { wizard.next() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!state.canGoNext)
                    .accessibilityIdentifier("form.next")
            }
        }
    }
}

private struct SummaryStep: View {
    let state: FormWizardState
    let wizard: FormWizardViewModelBridge

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Summary")
                .font(.headline)
                .accessibilityIdentifier("form.step")
            Text(state.summary)
                .font(.title3)
                .accessibilityIdentifier("form.summary")
            if state.isComplete {
                Text("Complete")
                    .font(.headline)
                    .foregroundStyle(.green)
                    .accessibilityIdentifier("form.complete")
            }
            HStack(spacing: 12) {
                Button("Back") { wizard.back() }
                    .accessibilityIdentifier("form.back")
                Button("Create account") { wizard.next() }
                    .buttonStyle(.borderedProminent)
                    .disabled(!state.canGoNext)
                    .accessibilityIdentifier("form.submit")
            }
        }
    }
}

private struct FieldError: View {
    let text: String?
    let id: String

    var body: some View {
        if let text {
            Text(text)
                .font(.caption)
                .foregroundStyle(.red)
                .accessibilityIdentifier(id)
        }
    }
}

#Preview {
    ContentView()
}
