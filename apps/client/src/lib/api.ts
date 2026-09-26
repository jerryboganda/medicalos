import { auth, clearToken } from './auth.svelte';
import type { LoginRequest } from './generated/auth/LoginRequest';
import type { LoginResponse } from './generated/auth/LoginResponse';
import type { RegisterRequest } from './generated/auth/RegisterRequest';
import type { RegisterResponse } from './generated/auth/RegisterResponse';
import type { Engagement } from './generated/engagement/Engagement';
import type { EngagementSettings } from './generated/engagement/EngagementSettings';
import type { EngagementSettingsResponse } from './generated/engagement/EngagementSettingsResponse';
import type { CompetitionListResponse } from './generated/engagement/CompetitionListResponse';
import type { StartCompetitionEntryRequest } from './generated/engagement/StartCompetitionEntryRequest';
import type { AnswerCompetitionQuestionRequest } from './generated/engagement/AnswerCompetitionQuestionRequest';
import type { CompetitionAttemptStep } from './generated/engagement/CompetitionAttemptStep';
import type { CompetitionLeaderboardResponse } from './generated/engagement/CompetitionLeaderboardResponse';
import type { CompetitionLeagueState } from './generated/engagement/CompetitionLeagueState';
import type { LeaveCompetitionLeagueResponse } from './generated/engagement/LeaveCompetitionLeagueResponse';
import type { QotdAnswerRequest } from './generated/engagement/QotdAnswerRequest';
import type { QotdAnswerResponse } from './generated/engagement/QotdAnswerResponse';
import type { CompleteOidcRequest } from './generated/oidc/CompleteOidcRequest';
import type { CompleteOidcResponse } from './generated/oidc/CompleteOidcResponse';
import type { OidcProviderUpdate } from './generated/oidc/OidcProviderUpdate';
import type { OidcProviderView } from './generated/oidc/OidcProviderView';
import type { StartInstitutionSsoResponse } from './generated/oidc/StartInstitutionSsoResponse';
import type { CreateNoteResponse } from './generated/notes/CreateNoteResponse';
import type { DeleteNoteResponse } from './generated/notes/DeleteNoteResponse';
import type { ListNotesResponse } from './generated/notes/ListNotesResponse';
import type { NoteRequest } from './generated/notes/NoteRequest';
import type { UpdateNoteResponse } from './generated/notes/UpdateNoteResponse';
import type { NextActionRecommendation } from './generated/today/NextActionRecommendation';
import type { NextActionRequest } from './generated/today/NextActionRequest';
import type { ReplanRequest } from './generated/today/ReplanRequest';
import type { ReplanResponse } from './generated/today/ReplanResponse';
import type { UndoResponse } from './generated/today/UndoResponse';
import type { TaskProtectionRequest } from './generated/today/TaskProtectionRequest';
import type { TaskProtectionResponse } from './generated/today/TaskProtectionResponse';
import type { CreateMockRequest } from './generated/mock/CreateMockRequest';
import type { CreateMockResponse } from './generated/mock/CreateMockResponse';
import type { MockListResponse } from './generated/mock/MockListResponse';
import type { StartMockResponse } from './generated/mock/StartMockResponse';
import type { SubmitResult } from './generated/mock/SubmitResult';
import type { CreateSessionResponse } from './generated/practice/CreateSessionResponse';
import type { CreateSessionRequest } from './generated/practice/CreateSessionRequest';
import type { AnswerRequest } from './generated/practice/AnswerRequest';
import type { AnswerResponse } from './generated/practice/AnswerResponse';
import type { PracticeSession } from './generated/practice/PracticeSession';
import type { AdminDashboard } from './generated/admin/AdminDashboard';
import type { AdminHierarchyResponse } from './generated/admin/AdminHierarchyResponse';
import type { CreateNodeRequest } from './generated/admin/CreateNodeRequest';
import type { CreateNodeResponse } from './generated/admin/CreateNodeResponse';
import type { AdminConceptListResponse } from './generated/concepts/AdminConceptListResponse';
import type { CreateConceptRequest } from './generated/concepts/CreateConceptRequest';
import type { CreateConceptResponse } from './generated/concepts/CreateConceptResponse';
import type { CreateConceptVersionRequest } from './generated/concepts/CreateConceptVersionRequest';
import type { CreateConceptVersionResponse } from './generated/concepts/CreateConceptVersionResponse';
import type { NodeConceptsResponse } from './generated/concepts/NodeConceptsResponse';
import type { SetNodeConceptsRequest } from './generated/concepts/SetNodeConceptsRequest';
import type { SetNodeConceptsResponse } from './generated/concepts/SetNodeConceptsResponse';
import type { AdminSettingsResponse } from './generated/settings/AdminSettingsResponse';
import type { AdminSettingsUpdateRequest } from './generated/settings/AdminSettingsUpdateRequest';
import type { AdminSettingsUpdateResponse } from './generated/settings/AdminSettingsUpdateResponse';
import type { AdminAuditResponse } from './generated/admin/AdminAuditResponse';
import type { AdminImportRequest } from './generated/admin/AdminImportRequest';
import type { AdminImportResponse } from './generated/admin/AdminImportResponse';
import type { AssessmentWorkflowRequest } from './generated/admin/AssessmentWorkflowRequest';
import type { AssessmentWorkflowResponse } from './generated/admin/AssessmentWorkflowResponse';
import type { RollbackImportResponse } from './generated/admin/RollbackImportResponse';
import type { AdminExtractionReport } from './generated/admin/AdminExtractionReport';
import type { AdminExtractionReportListResponse } from './generated/admin/AdminExtractionReportListResponse';
import type { ContentRightsRequest } from './generated/admin/ContentRightsRequest';
import type { ContentRightsResponse } from './generated/admin/ContentRightsResponse';
import type { CreateContentRightsResponse } from './generated/admin/CreateContentRightsResponse';
import type { CreateExtractionReportRequest } from './generated/admin/CreateExtractionReportRequest';
import type { ReviewExtractionReportRequest } from './generated/admin/ReviewExtractionReportRequest';
import type { RevokeContentRightsRequest } from './generated/admin/RevokeContentRightsRequest';
import type { RevokeContentRightsResponse } from './generated/admin/RevokeContentRightsResponse';
import type { AddInstitutionMemberRequest } from './generated/institutions/AddInstitutionMemberRequest';
import type { AddInstitutionMemberResponse } from './generated/institutions/AddInstitutionMemberResponse';
import type { CreateAssignmentRequest } from './generated/institutions/CreateAssignmentRequest';
import type { CreateAssignmentResponse } from './generated/institutions/CreateAssignmentResponse';
import type { CreateCohortRequest } from './generated/institutions/CreateCohortRequest';
import type { CreateCohortResponse } from './generated/institutions/CreateCohortResponse';
import type { CreateInstitutionRequest } from './generated/institutions/CreateInstitutionRequest';
import type { CreateInstitutionResponse } from './generated/institutions/CreateInstitutionResponse';
import type { CreateProgramRequest } from './generated/institutions/CreateProgramRequest';
import type { CreateProgramResponse } from './generated/institutions/CreateProgramResponse';
import type { InstitutionAnalyticsQuery } from './generated/institutions/InstitutionAnalyticsQuery';
import type { InstitutionAnalyticsResponse } from './generated/institutions/InstitutionAnalyticsResponse';
import type { InstitutionAuditResponse } from './generated/institutions/InstitutionAuditResponse';
import type { InstitutionCohortsResponse } from './generated/institutions/InstitutionCohortsResponse';
import type { InstitutionProgramsResponse } from './generated/institutions/InstitutionProgramsResponse';
import type { MyInstitutionsResponse } from './generated/institutions/MyInstitutionsResponse';
import type { ProgramCurriculumCoverageResponse } from './generated/institutions/ProgramCurriculumCoverageResponse';
import type { SetProgramCurriculumRequest } from './generated/institutions/SetProgramCurriculumRequest';
import type { SetProgramCurriculumResponse } from './generated/institutions/SetProgramCurriculumResponse';
import type { AdminArticleListResponse } from './generated/library/AdminArticleListResponse';
import type { AdminArticleVersion } from './generated/library/AdminArticleVersion';
import type { CreateArticleRequest } from './generated/library/CreateArticleRequest';
import type { UpdateArticleDraftRequest } from './generated/library/UpdateArticleDraftRequest';
import type { PublishArticleResponse } from './generated/library/PublishArticleResponse';
import type { LibraryArticleResponse } from './generated/library/LibraryArticleResponse';
import type { LibrarySearchResponse } from './generated/library/LibrarySearchResponse';
import type { PrivateDocumentSearchResult } from './generated/library/PrivateDocumentSearchResult';
import type { PrivateImportRightsResponse } from './generated/library/PrivateImportRightsResponse';
import type { PrivateImportSummary } from './generated/library/PrivateImportSummary';
import type { PrivateImportListResponse } from './generated/library/PrivateImportListResponse';
import type { CreatePrivateImportRequest } from './generated/library/CreatePrivateImportRequest';
import type { PrivateImportResponse } from './generated/library/PrivateImportResponse';
import type { PrivateImportDeletedResponse } from './generated/library/PrivateImportDeletedResponse';
import type { AdminImageAnnotationListResponse } from './generated/image/AdminImageAnnotationListResponse';
import type { ImageAnnotationRequest } from './generated/image/ImageAnnotationRequest';
import type { ImageAnnotationCreatedResponse } from './generated/image/ImageAnnotationCreatedResponse';
import type { ImageAnnotationReviewRequest } from './generated/image/ImageAnnotationReviewRequest';
import type { ImageAnnotationReviewResponse } from './generated/image/ImageAnnotationReviewResponse';
import type { ImageCaseConceptsRequest } from './generated/image/ImageCaseConceptsRequest';
import type { ImageCaseConceptsResponse } from './generated/image/ImageCaseConceptsResponse';
import type { ImageCaseDetail } from './generated/image/ImageCaseDetail';
import type { ImageCaseListResponse } from './generated/image/ImageCaseListResponse';
import type { ScenarioHandover } from './generated/scenario/ScenarioHandover';
import type { ScenarioHandoverAcknowledgedResponse } from './generated/scenario/ScenarioHandoverAcknowledgedResponse';
import type { ScenarioHandoverCreatedResponse } from './generated/scenario/ScenarioHandoverCreatedResponse';
import type { ScenarioHandoversResponse } from './generated/scenario/ScenarioHandoversResponse';
import type { ScenarioCounterfactualReplay } from './generated/scenario/ScenarioCounterfactualReplay';
import type { ScenarioAssessmentAppeal } from './generated/scenario/ScenarioAssessmentAppeal';
import type { ScenarioAssessmentAppealCreatedResponse } from './generated/scenario/ScenarioAssessmentAppealCreatedResponse';
import type { ScenarioAssessmentAppealQueueResponse } from './generated/scenario/ScenarioAssessmentAppealQueueResponse';
import type { ScenarioAssessmentAppealRequest } from './generated/scenario/ScenarioAssessmentAppealRequest';
import type { ScenarioAssessmentAppealReviewRequest } from './generated/scenario/ScenarioAssessmentAppealReviewRequest';
import type { ScenarioAssessmentAppealReviewResponse } from './generated/scenario/ScenarioAssessmentAppealReviewResponse';
import type { AdminScenarioAssessment } from './generated/scenario/AdminScenarioAssessment';
import type { PendingScenarioAssessmentsResponse } from './generated/scenario/PendingScenarioAssessmentsResponse';
import type { ScenarioAssessmentReceipt } from './generated/scenario/ScenarioAssessmentReceipt';
import type { ScenarioAssessmentRequest } from './generated/scenario/ScenarioAssessmentRequest';
import type { ScenarioDebrief } from './generated/scenario/ScenarioDebrief';
import type { ScenarioEventResponse } from './generated/scenario/ScenarioEventResponse';
import type { ScenarioListResponse } from './generated/scenario/ScenarioListResponse';
import type { ScenarioRun } from './generated/scenario/ScenarioRun';
import type { ScenarioStartResponse } from './generated/scenario/ScenarioStartResponse';
import type { ScenarioTimelineEvent } from './generated/scenario/ScenarioTimelineEvent';
import type { ScenarioTeam } from './generated/scenario/ScenarioTeam';
import type { ScenarioTeamInviteCreatedResponse } from './generated/scenario/ScenarioTeamInviteCreatedResponse';
import type { ScenarioTeamJoinResponse } from './generated/scenario/ScenarioTeamJoinResponse';
import type { ScenarioTeamRole } from './generated/scenario/ScenarioTeamRole';

import type { Today } from './generated/today/Today';
import type { MyCurriculumResponse } from './generated/today/MyCurriculumResponse';
import type { PackLeaseListResponse } from './generated/packs/PackLeaseListResponse';
import type { PackLeaseRequest } from './generated/packs/PackLeaseRequest';
import type { PackLeaseResponse } from './generated/packs/PackLeaseResponse';
import type { PackLeaseRevokedResponse } from './generated/packs/PackLeaseRevokedResponse';
import type { PackManifest } from './generated/packs/PackManifest';
import type { PackQuestionResource } from './generated/packs/PackQuestionResource';
import type { PackResourcesRequest } from './generated/packs/PackResourcesRequest';
import type { PackResourcesResponse } from './generated/packs/PackResourcesResponse';
import type { CeActivityRequest } from './generated/career/CeActivityRequest';
import type { CeActivityResponse } from './generated/career/CeActivityResponse';
import type { PortfolioCreateRequest } from './generated/career/PortfolioCreateRequest';
import type { PortfolioCreateResponse } from './generated/career/PortfolioCreateResponse';
import type { PortfolioListResponse } from './generated/career/PortfolioListResponse';
import type { NotificationInboxResponse } from './generated/inbox/NotificationInboxResponse';
import type { NotificationPreferencesUpdateRequest } from './generated/inbox/NotificationPreferencesUpdateRequest';
import type { NotificationPreferencesUpdateResponse } from './generated/inbox/NotificationPreferencesUpdateResponse';
import type { MarkResponse } from './generated/marks/MarkResponse';
import type { MarkedQuestionsResponse } from './generated/marks/MarkedQuestionsResponse';
import type { AddCardRequest } from './generated/review/AddCardRequest';
import type { AddCardResponse } from './generated/review/AddCardResponse';
import type { CreateDeckRequest } from './generated/review/CreateDeckRequest';
import type { CreateDeckResponse } from './generated/review/CreateDeckResponse';
import type { ReviewDebtResponse } from './generated/review/ReviewDebtResponse';
import type { ReviewEventRequest } from './generated/review/ReviewEventRequest';
import type { ReviewEventResponse } from './generated/review/ReviewEventResponse';
import type { ReviewQueueResponse } from './generated/review/ReviewQueueResponse';
import type { CalculatorInput } from './generated/calculators/CalculatorInput';
import type { CalculatorResponse } from './generated/calculators/CalculatorResponse';
import type { ConvertInput } from './generated/calculators/ConvertInput';
import type { ConvertResponse } from './generated/calculators/ConvertResponse';
import type { SessionHintResponse } from './generated/practice/SessionHintResponse';
import type { IntegrityEventRequest } from './generated/integrity/IntegrityEventRequest';
import type { IntegrityEventResponse } from './generated/integrity/IntegrityEventResponse';
import type { ExamRegistryResponse } from './generated/exams/ExamRegistryResponse';
import type { CoachTurnRequest } from './generated/coach/CoachTurnRequest';
import type { CoachTurnResponse } from './generated/coach/CoachTurnResponse';
import type { CoachHistoryResponse } from './generated/coach/CoachHistoryResponse';
import type { AnswerableQuestionsResponse } from './generated/coach/AnswerableQuestionsResponse';
import type { HeatmapQuery } from './generated/insights/HeatmapQuery';
import type { MasteryHeatmapResponse } from './generated/insights/MasteryHeatmapResponse';
import type { SelectionPolicyResponse } from './generated/program/SelectionPolicyResponse';
import type { CommunityProfileRequest } from './generated/community/CommunityProfileRequest';
import type { CommunityProfileResponse } from './generated/community/CommunityProfileResponse';
import type { CommunityProfileByHandleResponse } from './generated/community/CommunityProfileByHandleResponse';
import type { CreateCommunityProfileResponse } from './generated/community/CreateCommunityProfileResponse';
import type { CreateCommunityGroupRequest } from './generated/community/CreateCommunityGroupRequest';
import type { CreateCommunityGroupResponse } from './generated/community/CreateCommunityGroupResponse';
import type { JoinCommunityGroupResponse } from './generated/community/JoinCommunityGroupResponse';
import type { CreateCommunityPostRequest } from './generated/community/CreateCommunityPostRequest';
import type { CreateCommunityPostResponse } from './generated/community/CreateCommunityPostResponse';
import type { CommunityPostListResponse } from './generated/community/CommunityPostListResponse';
import type { ReportCommunityPostRequest } from './generated/community/ReportCommunityPostRequest';
import type { ReportCommunityPostResponse } from './generated/community/ReportCommunityPostResponse';
import type { MyCommunityPostReportsResponse } from './generated/community/MyCommunityPostReportsResponse';
import type { GroupPostReportQueueResponse } from './generated/community/GroupPostReportQueueResponse';
import type { ResolveCommunityPostReportRequest } from './generated/community/ResolveCommunityPostReportRequest';
import type { ResolveCommunityPostReportResponse } from './generated/community/ResolveCommunityPostReportResponse';
import type { RemoveCommunityPostResponse } from './generated/community/RemoveCommunityPostResponse';
import type { CommunityGroupsResponse } from './generated/community/CommunityGroupsResponse';
import type { CreateDuelRequest } from './generated/community/CreateDuelRequest';
import type { CreateDuelResponse } from './generated/community/CreateDuelResponse';
import type { DuelByTokenResponse } from './generated/community/DuelByTokenResponse';
import type { AcceptDuelResponse } from './generated/community/AcceptDuelResponse';
import type { DuelStateResponse } from './generated/community/DuelStateResponse';
import type { DeclineDuelResponse } from './generated/community/DeclineDuelResponse';
import type { MyDuelsResponse } from './generated/community/MyDuelsResponse';
import type { ShareCardsResponse } from './generated/community/ShareCardsResponse';
import type { ReportQuestionRequest } from './generated/reports/ReportQuestionRequest';
import type { QuestionReportResponse } from './generated/reports/QuestionReportResponse';
import type { AdminReportsResponse } from './generated/reports/AdminReportsResponse';
import type { ResolveQuestionReportRequest } from './generated/reports/ResolveQuestionReportRequest';
import type { ResolveQuestionReportResponse } from './generated/reports/ResolveQuestionReportResponse';

// Keep transport and authentication here. Data contracts are exported from
// Rust DTOs incrementally as ARCH-02 bindings are generated.
// Production reaches the API same-origin through the /api/ prefix
// (medicalos.polytronx.com/api/* -> the API container); local dev keeps
// talking straight to the API host.
export type { LoginRequest } from './generated/auth/LoginRequest';
export type { LoginResponse } from './generated/auth/LoginResponse';
export type { RegisterRequest } from './generated/auth/RegisterRequest';
export type { RegisterResponse } from './generated/auth/RegisterResponse';
export type { Engagement } from './generated/engagement/Engagement';
export type { CompetitionSummary } from './generated/engagement/CompetitionSummary';
export type { CompetitionListResponse } from './generated/engagement/CompetitionListResponse';
export type { StartCompetitionEntryRequest } from './generated/engagement/StartCompetitionEntryRequest';
export type { AnswerCompetitionQuestionRequest } from './generated/engagement/AnswerCompetitionQuestionRequest';
export type { CompetitionQuestionOption } from './generated/engagement/CompetitionQuestionOption';
export type { CompetitionQuestion } from './generated/engagement/CompetitionQuestion';
export type { CompetitionInProgressResponse } from './generated/engagement/CompetitionInProgressResponse';
export type { CompetitionSubmittedResponse } from './generated/engagement/CompetitionSubmittedResponse';
export type { CompetitionAttemptStep } from './generated/engagement/CompetitionAttemptStep';
export type { CompetitionLeaderboardEntry } from './generated/engagement/CompetitionLeaderboardEntry';
export type { CompetitionLeaderboardResponse } from './generated/engagement/CompetitionLeaderboardResponse';
export type { CompetitionLeagueNotJoined } from './generated/engagement/CompetitionLeagueNotJoined';
export type { CompetitionLeagueStanding } from './generated/engagement/CompetitionLeagueStanding';
export type { CompetitionLeagueJoined } from './generated/engagement/CompetitionLeagueJoined';
export type { CompetitionLeagueState } from './generated/engagement/CompetitionLeagueState';
export type { LeaveCompetitionLeagueResponse } from './generated/engagement/LeaveCompetitionLeagueResponse';
export type { EngagementDailyGoal } from './generated/engagement/EngagementDailyGoal';
export type { EngagementStreak } from './generated/engagement/EngagementStreak';
export type { EngagementOption } from './generated/engagement/EngagementOption';
export type { EngagementCommunitySplit } from './generated/engagement/EngagementCommunitySplit';
export type { EngagementQotd } from './generated/engagement/EngagementQotd';
export type { EngagementSettings } from './generated/engagement/EngagementSettings';
export type { EngagementSettingsResponse } from './generated/engagement/EngagementSettingsResponse';
export type { QotdAnswerRequest } from './generated/engagement/QotdAnswerRequest';
export type { QotdAnswerResponse } from './generated/engagement/QotdAnswerResponse';
export type { CompleteOidcRequest } from './generated/oidc/CompleteOidcRequest';
export type { CompleteOidcResponse } from './generated/oidc/CompleteOidcResponse';
export type { OidcProviderUpdate } from './generated/oidc/OidcProviderUpdate';
export type { OidcProviderView } from './generated/oidc/OidcProviderView';
export type { StartInstitutionSsoResponse } from './generated/oidc/StartInstitutionSsoResponse';
export type { CeActivityRequest } from './generated/career/CeActivityRequest';
export type { CeActivityResponse } from './generated/career/CeActivityResponse';
export type { PortfolioCreateRequest } from './generated/career/PortfolioCreateRequest';
export type { PortfolioCreateResponse } from './generated/career/PortfolioCreateResponse';
export type { PortfolioEntry } from './generated/career/PortfolioEntry';
export type { PortfolioListResponse } from './generated/career/PortfolioListResponse';
export type { NotificationInboxResponse } from './generated/inbox/NotificationInboxResponse';
export type { NotificationItem } from './generated/inbox/NotificationItem';
export type { NotificationPreferences } from './generated/inbox/NotificationPreferences';
export type { NotificationPreferencesUpdateRequest } from './generated/inbox/NotificationPreferencesUpdateRequest';
export type { NotificationPreferencesUpdateResponse } from './generated/inbox/NotificationPreferencesUpdateResponse';
export type { Today } from './generated/today/Today';
export type { TodayTask } from './generated/today/TodayTask';
export type { TodayRevision } from './generated/today/TodayRevision';
export type { LearnerChapter } from './generated/today/LearnerChapter';
export type { TodayRevisionBudget } from './generated/today/TodayRevisionBudget';
export type { NextActionAllowance } from './generated/today/NextActionAllowance';
export type { NextActionDeadlinePassed } from './generated/today/NextActionDeadlinePassed';
export type { NextActionNoAction } from './generated/today/NextActionNoAction';
export type { NextActionNoPlan } from './generated/today/NextActionNoPlan';
export type { NextActionRecommendation } from './generated/today/NextActionRecommendation';
export type { NextActionRequest } from './generated/today/NextActionRequest';
export type { NextActionWithAction } from './generated/today/NextActionWithAction';
export type { RecommendedAction } from './generated/today/RecommendedAction';
export type { ReplanAppliedResponse } from './generated/today/ReplanAppliedResponse';
export type { ReplanRequest } from './generated/today/ReplanRequest';
export type { ReplanResponse } from './generated/today/ReplanResponse';
export type { UndoResponse } from './generated/today/UndoResponse';
export type { ReplanWithinCapacityResponse } from './generated/today/ReplanWithinCapacityResponse';
export type { TaskProtectionRequest } from './generated/today/TaskProtectionRequest';
export type { TaskProtectionResponse } from './generated/today/TaskProtectionResponse';
export type { CurriculumChapter } from './generated/today/CurriculumChapter';
export type { MyCurriculumResponse } from './generated/today/MyCurriculumResponse';
export type { AdminDashboard } from './generated/admin/AdminDashboard';
export type { AdminAuditEvent } from './generated/admin/AdminAuditEvent';
export type { AdminAuditResponse } from './generated/admin/AdminAuditResponse';
export type { AdminImportAppliedResponse } from './generated/admin/AdminImportAppliedResponse';
export type { AdminImportIssue } from './generated/admin/AdminImportIssue';
export type { AdminImportedQuestion } from './generated/admin/AdminImportedQuestion';
export type { AdminImportPreviewResponse } from './generated/admin/AdminImportPreviewResponse';
export type { AdminImportRequest } from './generated/admin/AdminImportRequest';
export type { AdminImportResponse } from './generated/admin/AdminImportResponse';
export type { AssessmentWorkflowError } from './generated/admin/AssessmentWorkflowError';
export type { AssessmentWorkflowRequest } from './generated/admin/AssessmentWorkflowRequest';
export type { AssessmentWorkflowResponse } from './generated/admin/AssessmentWorkflowResponse';
export type { AssessmentWorkflowResult } from './generated/admin/AssessmentWorkflowResult';
export type { ImportQuestion } from './generated/admin/ImportQuestion';
export type { RollbackImportResponse } from './generated/admin/RollbackImportResponse';
export type { AdminCurriculumNode } from './generated/admin/AdminCurriculumNode';
export type { AdminHierarchyResponse } from './generated/admin/AdminHierarchyResponse';
export type { CreateNodeRequest } from './generated/admin/CreateNodeRequest';
export type { CreateNodeResponse } from './generated/admin/CreateNodeResponse';
export type { AdminConcept } from './generated/concepts/AdminConcept';
export type { AdminConceptListResponse } from './generated/concepts/AdminConceptListResponse';
export type { CreateConceptRequest } from './generated/concepts/CreateConceptRequest';
export type { CreateConceptResponse } from './generated/concepts/CreateConceptResponse';
export type { CreateConceptVersionRequest } from './generated/concepts/CreateConceptVersionRequest';
export type { CreateConceptVersionResponse } from './generated/concepts/CreateConceptVersionResponse';
export type { NodeConcept } from './generated/concepts/NodeConcept';
export type { NodeConceptsResponse } from './generated/concepts/NodeConceptsResponse';
export type { SetNodeConceptsRequest } from './generated/concepts/SetNodeConceptsRequest';
export type { SetNodeConceptsResponse } from './generated/concepts/SetNodeConceptsResponse';
export type { AdminSettings } from './generated/settings/AdminSettings';
export type { AdminSettingsResponse } from './generated/settings/AdminSettingsResponse';
export type { AdminSettingsUpdateRequest } from './generated/settings/AdminSettingsUpdateRequest';
export type { AdminSettingsUpdateResponse } from './generated/settings/AdminSettingsUpdateResponse';
export type { AdminContentRight } from './generated/admin/AdminContentRight';
export type { AdminExtractionReport } from './generated/admin/AdminExtractionReport';
export type { AdminExtractionReportListResponse } from './generated/admin/AdminExtractionReportListResponse';
export type { ContentRightStatus } from './generated/admin/ContentRightStatus';
export type { ContentRightsRequest } from './generated/admin/ContentRightsRequest';
export type { ContentRightsResponse } from './generated/admin/ContentRightsResponse';
export type { CreateContentRightsResponse } from './generated/admin/CreateContentRightsResponse';
export type { CreateExtractionReportRequest } from './generated/admin/CreateExtractionReportRequest';
export type { ExtractionMalwareScanStatus } from './generated/admin/ExtractionMalwareScanStatus';
export type { ExtractionReportReview } from './generated/admin/ExtractionReportReview';
export type { ExtractionReportStatus } from './generated/admin/ExtractionReportStatus';
export type { ExtractionReviewDecision } from './generated/admin/ExtractionReviewDecision';
export type { AddInstitutionMemberRequest } from './generated/institutions/AddInstitutionMemberRequest';
export type { AddInstitutionMemberResponse } from './generated/institutions/AddInstitutionMemberResponse';
export type { CreateAssignmentRequest } from './generated/institutions/CreateAssignmentRequest';
export type { CreateAssignmentResponse } from './generated/institutions/CreateAssignmentResponse';
export type { CreateCohortRequest } from './generated/institutions/CreateCohortRequest';
export type { CreateCohortResponse } from './generated/institutions/CreateCohortResponse';
export type { CreateInstitutionRequest } from './generated/institutions/CreateInstitutionRequest';
export type { CreateInstitutionResponse } from './generated/institutions/CreateInstitutionResponse';
export type { CreateProgramRequest } from './generated/institutions/CreateProgramRequest';
export type { CreateProgramResponse } from './generated/institutions/CreateProgramResponse';
export type { InstitutionAnalyticsChapter } from './generated/institutions/InstitutionAnalyticsChapter';
export type { InstitutionAnalyticsQuery } from './generated/institutions/InstitutionAnalyticsQuery';
export type { InstitutionAnalyticsResponse } from './generated/institutions/InstitutionAnalyticsResponse';
export type { InstitutionAuditEvent } from './generated/institutions/InstitutionAuditEvent';
export type { InstitutionAuditResponse } from './generated/institutions/InstitutionAuditResponse';
export type { CohortSummary } from './generated/institutions/CohortSummary';
export type { InstitutionCohortsResponse } from './generated/institutions/InstitutionCohortsResponse';
export type { InstitutionMembership } from './generated/institutions/InstitutionMembership';
export type { InstitutionProgram } from './generated/institutions/InstitutionProgram';
export type { InstitutionProgramsResponse } from './generated/institutions/InstitutionProgramsResponse';
export type { MyInstitutionsResponse } from './generated/institutions/MyInstitutionsResponse';
export type { ProgramCurriculumCoverageChapter } from './generated/institutions/ProgramCurriculumCoverageChapter';
export type { ProgramCurriculumCoverageResponse } from './generated/institutions/ProgramCurriculumCoverageResponse';
export type { SetProgramCurriculumRequest } from './generated/institutions/SetProgramCurriculumRequest';
export type { SetProgramCurriculumResponse } from './generated/institutions/SetProgramCurriculumResponse';
export type { ReviewExtractionReportRequest } from './generated/admin/ReviewExtractionReportRequest';
export type { RevokeContentRightsRequest } from './generated/admin/RevokeContentRightsRequest';
export type { RevokeContentRightsResponse } from './generated/admin/RevokeContentRightsResponse';
export type { ExtractionMalwareScanStatus } from './generated/admin/ExtractionMalwareScanStatus';
export type { ExtractionReportReview } from './generated/admin/ExtractionReportReview';
export type { ExtractionReportStatus } from './generated/admin/ExtractionReportStatus';
export type { ExtractionReviewDecision } from './generated/admin/ExtractionReviewDecision';
export type { MasteryHeatmapBand } from './generated/insights/MasteryHeatmapBand';
export type { MasteryHeatmapBands } from './generated/insights/MasteryHeatmapBands';
export type { MasteryHeatmapChapter } from './generated/insights/MasteryHeatmapChapter';
export type { MasteryHeatmapSystem } from './generated/insights/MasteryHeatmapSystem';
export type { MasteryHeatmapResponse } from './generated/insights/MasteryHeatmapResponse';
export type { HeatmapQuery } from './generated/insights/HeatmapQuery';
export type { SelectionPolicyChapter } from './generated/program/SelectionPolicyChapter';
export type { SelectionPolicyEstimator } from './generated/program/SelectionPolicyEstimator';
export type { SelectionPolicyResponse } from './generated/program/SelectionPolicyResponse';
export type { CommunityProfileRequest } from './generated/community/CommunityProfileRequest';
export type { CommunityProfileResponse } from './generated/community/CommunityProfileResponse';
export type { CommunityProfileByHandleResponse } from './generated/community/CommunityProfileByHandleResponse';
export type { CreateCommunityProfileResponse } from './generated/community/CreateCommunityProfileResponse';
export type { CreateCommunityGroupRequest } from './generated/community/CreateCommunityGroupRequest';
export type { CreateCommunityGroupResponse } from './generated/community/CreateCommunityGroupResponse';
export type { JoinCommunityGroupResponse } from './generated/community/JoinCommunityGroupResponse';
export type { CreateCommunityPostRequest } from './generated/community/CreateCommunityPostRequest';
export type { CreateCommunityPostResponse } from './generated/community/CreateCommunityPostResponse';
export type { CommunityPost } from './generated/community/CommunityPost';
export type { CommunityPostListResponse } from './generated/community/CommunityPostListResponse';
export type { ReportCommunityPostRequest } from './generated/community/ReportCommunityPostRequest';
export type { ReportCommunityPostResponse } from './generated/community/ReportCommunityPostResponse';
export type { CommunityPostReportSummary } from './generated/community/CommunityPostReportSummary';
export type { MyCommunityPostReportsResponse } from './generated/community/MyCommunityPostReportsResponse';
export type { GroupPostReport } from './generated/community/GroupPostReport';
export type { GroupPostReportQueueResponse } from './generated/community/GroupPostReportQueueResponse';
export type { ResolveCommunityPostReportRequest } from './generated/community/ResolveCommunityPostReportRequest';
export type { ResolveCommunityPostReportResponse } from './generated/community/ResolveCommunityPostReportResponse';
export type { RemoveCommunityPostResponse } from './generated/community/RemoveCommunityPostResponse';
export type { CommunityGroupSummary } from './generated/community/CommunityGroupSummary';
export type { CommunityGroupsResponse } from './generated/community/CommunityGroupsResponse';
export type { CreateDuelRequest } from './generated/community/CreateDuelRequest';
export type { CreateDuelResponse } from './generated/community/CreateDuelResponse';
export type { DuelByTokenResponse } from './generated/community/DuelByTokenResponse';
export type { AcceptDuelResponse } from './generated/community/AcceptDuelResponse';
export type { DuelSide } from './generated/community/DuelSide';
export type { DuelStateResponse } from './generated/community/DuelStateResponse';
export type { DeclineDuelResponse } from './generated/community/DeclineDuelResponse';
export type { DuelSummary } from './generated/community/DuelSummary';
export type { MyDuelsResponse } from './generated/community/MyDuelsResponse';
export type { ShareCardKind } from './generated/community/ShareCardKind';
export type { ShareCard } from './generated/community/ShareCard';
export type { UnavailableShareCard } from './generated/community/UnavailableShareCard';
export type { ShareCardsResponse } from './generated/community/ShareCardsResponse';
export type { ReportQuestionRequest } from './generated/reports/ReportQuestionRequest';
export type { QuestionReportResponse } from './generated/reports/QuestionReportResponse';
export type { ReportFeedback } from './generated/reports/ReportFeedback';
export type { AdminReport } from './generated/reports/AdminReport';
export type { AdminReportsResponse } from './generated/reports/AdminReportsResponse';
export type { ResolveQuestionReportRequest } from './generated/reports/ResolveQuestionReportRequest';
export type { ResolveQuestionReportResponse } from './generated/reports/ResolveQuestionReportResponse';
export type { AdminArticleListResponse } from './generated/library/AdminArticleListResponse';
export type { AdminArticleSummary } from './generated/library/AdminArticleSummary';
export type { AdminArticleVersion } from './generated/library/AdminArticleVersion';
export type { ArticleCitation } from './generated/library/ArticleCitation';
export type { CreateArticleRequest } from './generated/library/CreateArticleRequest';
export type { LibraryArticleResponse } from './generated/library/LibraryArticleResponse';
export type { LibrarySearchResponse } from './generated/library/LibrarySearchResponse';
export type { LibrarySearchResult } from './generated/library/LibrarySearchResult';
export type { UpdateArticleDraftRequest } from './generated/library/UpdateArticleDraftRequest';
export type { PublishArticleResponse } from './generated/library/PublishArticleResponse';
export type { PrivateDocumentSearchResult } from './generated/library/PrivateDocumentSearchResult';
export type { PrivateImportRight } from './generated/library/PrivateImportRight';
export type { PrivateImportRightsResponse } from './generated/library/PrivateImportRightsResponse';
export type { PrivateImportSummary } from './generated/library/PrivateImportSummary';
export type { PrivateImportListResponse } from './generated/library/PrivateImportListResponse';
export type { CreatePrivateImportRequest } from './generated/library/CreatePrivateImportRequest';
export type { PrivateImportResponse } from './generated/library/PrivateImportResponse';
export type { PrivateImportDeletedResponse } from './generated/library/PrivateImportDeletedResponse';
export type { ArticleMedia } from './generated/library/ArticleMedia';
export type { MediaCaptionCue } from './generated/library/MediaCaptionCue';
export type { MediaChapterMarker } from './generated/library/MediaChapterMarker';
export type { ImageConceptLink } from './generated/image/ImageConceptLink';
export type { AdminImageAnnotation } from './generated/image/AdminImageAnnotation';
export type { AdminImageAnnotationListResponse } from './generated/image/AdminImageAnnotationListResponse';
export type { ImageAnnotationRequest } from './generated/image/ImageAnnotationRequest';
export type { ImageAnnotationCreatedResponse } from './generated/image/ImageAnnotationCreatedResponse';
export type { ImageAnnotationDecision } from './generated/image/ImageAnnotationDecision';
export type { ImageAnnotationReviewRequest } from './generated/image/ImageAnnotationReviewRequest';
export type { ImageAnnotationReviewResponse } from './generated/image/ImageAnnotationReviewResponse';
export type { ImageCaseAnnotation } from './generated/image/ImageCaseAnnotation';
export type { ImageCaseConceptsResponse } from './generated/image/ImageCaseConceptsResponse';
export type { ImageCaseConceptsRequest } from './generated/image/ImageCaseConceptsRequest';
export type { ImageCaseDetail } from './generated/image/ImageCaseDetail';
export type { ImageCaseKind } from './generated/image/ImageCaseKind';
export type { ImageCaseListResponse } from './generated/image/ImageCaseListResponse';
export type { ImageCaseSummary } from './generated/image/ImageCaseSummary';
export type { ImageFinding } from './generated/image/ImageFinding';
export type { ImageRef } from './generated/image/ImageRef';
export type { PendingImageAnnotationStatus } from './generated/image/PendingImageAnnotationStatus';
export type { ScenarioListResponse } from './generated/scenario/ScenarioListResponse';
export type { ScenarioAppealSummary } from './generated/scenario/ScenarioAppealSummary';
export type { ScenarioAppealDecision } from './generated/scenario/ScenarioAppealDecision';
export type { ScenarioAppealStatus } from './generated/scenario/ScenarioAppealStatus';
export type { ScenarioAssessmentAppeal } from './generated/scenario/ScenarioAssessmentAppeal';
export type { ScenarioAssessmentAppealCreatedResponse } from './generated/scenario/ScenarioAssessmentAppealCreatedResponse';
export type { ScenarioAssessmentAppealQueueItem } from './generated/scenario/ScenarioAssessmentAppealQueueItem';
export type { ScenarioAssessmentAppealQueueResponse } from './generated/scenario/ScenarioAssessmentAppealQueueResponse';
export type { ScenarioAssessmentAppealRequest } from './generated/scenario/ScenarioAssessmentAppealRequest';
export type { ScenarioAssessmentAppealReviewRequest } from './generated/scenario/ScenarioAssessmentAppealReviewRequest';
export type { ScenarioAssessmentAppealReviewResponse } from './generated/scenario/ScenarioAssessmentAppealReviewResponse';
export type { AdminScenarioAssessment } from './generated/scenario/AdminScenarioAssessment';
export type { PendingScenarioAssessment } from './generated/scenario/PendingScenarioAssessment';
export type { PendingScenarioAssessmentsResponse } from './generated/scenario/PendingScenarioAssessmentsResponse';
export type { ScenarioAssessmentReceipt } from './generated/scenario/ScenarioAssessmentReceipt';
export type { ScenarioAssessmentRequest } from './generated/scenario/ScenarioAssessmentRequest';
export type { ScenarioAssessmentStatus } from './generated/scenario/ScenarioAssessmentStatus';
export type { ScenarioCriterionAssessment } from './generated/scenario/ScenarioCriterionAssessment';
export type { ScenarioCounterfactualEvent } from './generated/scenario/ScenarioCounterfactualEvent';
export type { ScenarioCounterfactualReplay } from './generated/scenario/ScenarioCounterfactualReplay';
export type { ScenarioConsequentialUseStatus } from './generated/scenario/ScenarioConsequentialUseStatus';
export type { ScenarioDebrief } from './generated/scenario/ScenarioDebrief';
export type { ScenarioEventResponse } from './generated/scenario/ScenarioEventResponse';
export type { ScenarioRubricResult } from './generated/scenario/ScenarioRubricResult';
export type { ScenarioRubricStatus } from './generated/scenario/ScenarioRubricStatus';
export type { ScenarioRun } from './generated/scenario/ScenarioRun';
export type { ScenarioStartResponse } from './generated/scenario/ScenarioStartResponse';
export type { ScenarioTimelineEvent } from './generated/scenario/ScenarioTimelineEvent';
export type { ScenarioTranscriptCorrection } from './generated/scenario/ScenarioTranscriptCorrection';
export type { ScenarioSummary } from './generated/scenario/ScenarioSummary';
export type { ScenarioHandover } from './generated/scenario/ScenarioHandover';
export type { ScenarioHandoverAcknowledgedResponse } from './generated/scenario/ScenarioHandoverAcknowledgedResponse';
export type { ScenarioHandoverCreatedResponse } from './generated/scenario/ScenarioHandoverCreatedResponse';
export type { ScenarioHandoversResponse } from './generated/scenario/ScenarioHandoversResponse';
export type { ScenarioTeam } from './generated/scenario/ScenarioTeam';
export type { ScenarioTeamInviteCreatedResponse } from './generated/scenario/ScenarioTeamInviteCreatedResponse';
export type { ScenarioTeamJoinResponse } from './generated/scenario/ScenarioTeamJoinResponse';
export type { ScenarioTeamMember } from './generated/scenario/ScenarioTeamMember';
export type { ScenarioTeamRole } from './generated/scenario/ScenarioTeamRole';
export type { PackLeaseListResponse } from './generated/packs/PackLeaseListResponse';
export type { PackLeaseRequest } from './generated/packs/PackLeaseRequest';
export type { PackLeaseResponse } from './generated/packs/PackLeaseResponse';
export type { PackLeaseRevokedResponse } from './generated/packs/PackLeaseRevokedResponse';
export type { PackLeaseSummary } from './generated/packs/PackLeaseSummary';
export type { PackManifest } from './generated/packs/PackManifest';
export type { PackManifestItem } from './generated/packs/PackManifestItem';
export type { PackQuestionResource } from './generated/packs/PackQuestionResource';
export type { PackResourcesRequest } from './generated/packs/PackResourcesRequest';
export type { PackResourcesResponse } from './generated/packs/PackResourcesResponse';
export type { QuestionOption } from './generated/packs/QuestionOption';
export type { TutoringCard } from './generated/packs/TutoringCard';
export type { CreateMockRequest } from './generated/mock/CreateMockRequest';
export type { CreateMockResponse } from './generated/mock/CreateMockResponse';
export type { MockBlueprintEntry } from './generated/mock/MockBlueprintEntry';
export type { MockIntegrityPolicy } from './generated/mock/MockIntegrityPolicy';
export type { MockListResponse } from './generated/mock/MockListResponse';
export type { MockResult } from './generated/mock/MockResult';
export type { MockResultBreakdown } from './generated/mock/MockResultBreakdown';
export type { MockTest } from './generated/mock/MockTest';
export type { MockType } from './generated/mock/MockType';
export type { StartMockResponse } from './generated/mock/StartMockResponse';
export type { SubmitResult } from './generated/mock/SubmitResult';
export type { SubmitTime } from './generated/mock/SubmitTime';
export type { SubmitTimeItem } from './generated/mock/SubmitTimeItem';
export type { CreateSessionItem } from './generated/practice/CreateSessionItem';
export type { CreateSessionResponse } from './generated/practice/CreateSessionResponse';
export type { BlueprintSlice } from './generated/practice/BlueprintSlice';
export type { CreateSessionRequest } from './generated/practice/CreateSessionRequest';
export type { AnswerRequest } from './generated/practice/AnswerRequest';
export type { AnswerFeedbackResponse } from './generated/practice/AnswerFeedbackResponse';
export type { AnswerRecordedResponse } from './generated/practice/AnswerRecordedResponse';
export type { AnswerResponse } from './generated/practice/AnswerResponse';
export type { PracticeSession } from './generated/practice/PracticeSession';
export type { SessionDetailOption } from './generated/practice/SessionDetailOption';
export type { SessionItem } from './generated/practice/SessionItem';
export type { SessionReportReceipt } from './generated/practice/SessionReportReceipt';
export type { SessionReportStatus } from './generated/practice/SessionReportStatus';
export type { SessionOption } from './generated/practice/SessionOption';
export type { Note } from './generated/notes/Note';
export type { NoteBacklink } from './generated/notes/NoteBacklink';
export type { NoteRequest } from './generated/notes/NoteRequest';
export type { CreateNoteResponse } from './generated/notes/CreateNoteResponse';
export type { UpdateNoteResponse } from './generated/notes/UpdateNoteResponse';
export type { ListNotesResponse } from './generated/notes/ListNotesResponse';
export type { DeleteNoteResponse } from './generated/notes/DeleteNoteResponse';
export type { MarkResponse } from './generated/marks/MarkResponse';
export type { MarkedQuestion } from './generated/marks/MarkedQuestion';
export type { MarkedQuestionsResponse } from './generated/marks/MarkedQuestionsResponse';
export type { AddCardRequest } from './generated/review/AddCardRequest';
export type { AddCardResponse } from './generated/review/AddCardResponse';
export type { CreateDeckRequest } from './generated/review/CreateDeckRequest';
export type { CreateDeckResponse } from './generated/review/CreateDeckResponse';
export type { ReviewDebtResponse } from './generated/review/ReviewDebtResponse';
export type { ReviewEventRequest } from './generated/review/ReviewEventRequest';
export type { ReviewEventResponse } from './generated/review/ReviewEventResponse';
export type { ReviewQueueItem } from './generated/review/ReviewQueueItem';
export type { ReviewQueueResponse } from './generated/review/ReviewQueueResponse';
export type { CalculatorInput } from './generated/calculators/CalculatorInput';
export type { CalculatorResponse } from './generated/calculators/CalculatorResponse';
export type { ConvertInput } from './generated/calculators/ConvertInput';
export type { ConvertResponse } from './generated/calculators/ConvertResponse';
export type { SessionHintResponse } from './generated/practice/SessionHintResponse';
export type { IntegrityEventRequest } from './generated/integrity/IntegrityEventRequest';
export type { IntegrityEventResponse } from './generated/integrity/IntegrityEventResponse';
export type { ExamRegistryItem } from './generated/exams/ExamRegistryItem';
export type { ExamRegistryResponse } from './generated/exams/ExamRegistryResponse';
export type { CoachTurnRequest } from './generated/coach/CoachTurnRequest';
export type { CoachTurnResponse } from './generated/coach/CoachTurnResponse';
export type { CoachHistoryTurn } from './generated/coach/CoachHistoryTurn';
export type { CoachHistoryResponse } from './generated/coach/CoachHistoryResponse';
export type { AnswerableQuestion } from './generated/coach/AnswerableQuestion';
export type { AnswerableQuestionsResponse } from './generated/coach/AnswerableQuestionsResponse';

export type SubmitReceipt = Omit<SubmitResult, 'expected_score' | 'time'> &
	Partial<Pick<SubmitResult, 'expected_score' | 'time'>>;
const BASE: string =
	import.meta.env.VITE_API_BASE ?? (import.meta.env.PROD ? '/api' : '');

export class ApiError extends Error {
	constructor(
		public status: number,
		public code: string,
		message: string
	) {
		super(message);
	}
}

export function apiErrorMessage(value: unknown, fallback: string): string {
	return value instanceof Error ? value.message : fallback;
}

export function adminToken(): string {
	try {
		return localStorage.getItem('mlos_admin') ?? '';
	} catch {
		return '';
	}
}

async function call<T>(
	method: string,
	path: string,
	body?: unknown,
	contentType = 'application/json'
): Promise<T> {
	const res = await fetch(`${BASE}${path}`, {
		method,
		headers: {
			'content-type': contentType,
			'x-admin-token': adminToken(),
			...(auth.token ? { authorization: `Bearer ${auth.token}` } : {})
		},
		body:
			body === undefined
				? undefined
				: contentType === 'application/json'
					? JSON.stringify(body)
					: (body as BodyInit)
	});
	if (!res.ok) {
		if (res.status === 401) {
			clearToken();
		}
		let code = 'error';
		let message = `Request failed (${res.status})`;
		try {
			const parsed = await res.json();
			code = parsed?.error?.code ?? code;
			message = parsed?.error?.message ?? message;
		} catch {
			/* non-json error body */
		}
		throw new ApiError(res.status, code, message);
	}
	return (await res.json()) as T;
}

export const Api = {
	register: (email: string, password: string) =>
		call<RegisterResponse>(
			'POST',
			'/v1/auth/register',
			{ email, password } satisfies RegisterRequest
		),
	login: (email: string, password: string) =>
		call<LoginResponse>(
			'POST',
			'/v1/auth/login',
			{ email, password } satisfies LoginRequest
		),
	getInstitutionOidc: (institutionId: string) =>
		call<OidcProviderView>(
			'GET',
			`/v1/admin/institutions/${encodeURIComponent(institutionId)}/sso/oidc`
		),
	configureInstitutionOidc: (institutionId: string, body: OidcProviderUpdate) =>
		call<OidcProviderView>(
			'PUT',
			`/v1/admin/institutions/${encodeURIComponent(institutionId)}/sso/oidc`,
			body
		),
	startInstitutionSso: (institutionId: string) =>
		call<StartInstitutionSsoResponse>(
			'GET',
			`/v1/institutions/${encodeURIComponent(institutionId)}/sso/oidc/start`
		),
	completeInstitutionSso: (ticket: string) =>
		call<CompleteOidcResponse>(
			'POST',
			'/v1/auth/oidc/complete',
			{ ticket } satisfies CompleteOidcRequest
		),
	today: () => call<Today>('GET', '/v1/me/today'),
	recommendNextAction: (
		availableMinutes: NextActionRequest['available_minutes'],
		activityPreference: NonNullable<NextActionRequest['activity_preference']> = 'any',
		timeMultiplier: NonNullable<NextActionRequest['time_multiplier']> = 1
	) => {
		const query = new URLSearchParams({
			available_minutes: String(availableMinutes),
			activity_preference: activityPreference,
			time_multiplier: String(timeMultiplier)
		});
		return call<NextActionRecommendation>('GET', `/v1/me/plan/next-action?${query}`);
	},
	protectPlanTask: (planId: string, taskId: string, isProtected: boolean) =>
		call<TaskProtectionResponse>(
			'PUT',
			`/v1/plans/${planId}/tasks/${taskId}/protection`,
			{ protected: isProtected } satisfies TaskProtectionRequest
		),
	replan: (dailyMinutes: number, expectedVersion: number) =>
		call<ReplanResponse>(
			'POST',
			'/v1/me/plan/replan',
			{ daily_minutes: dailyMinutes, expected_version: expectedVersion } satisfies ReplanRequest
		),
	engagement: () => call<Engagement>('GET', '/v1/me/engagement'),
	updateEngagementSettings: (body: EngagementSettings) =>
		call<EngagementSettingsResponse>('PUT', '/v1/me/engagement/settings', body),
	answerQotd: (questionVersionId: string, chosenIndex: number, elapsedMs = 0) => {
		const body: QotdAnswerRequest = {
			question_version_id: questionVersionId,
			chosen_index: chosenIndex,
			elapsed_ms: elapsedMs
		};
		return call<QotdAnswerResponse>('POST', '/v1/me/qotd/answers', body);
	},
	createSession: (body: CreateSessionRequest) =>
		call<CreateSessionResponse>('POST', '/v1/practice/sessions', body),
	createPackLease: (body: PackLeaseRequest) =>
		call<PackLeaseResponse>('POST', '/v1/packs/lease', body),
	packManifest: (examId: string, chapters: string[], deviceId: string) => {
		const query = new URLSearchParams({ chapters: chapters.join(','), device_id: deviceId });
		return call<PackManifest>(
			'GET',
			`/v2/packs/${encodeURIComponent(examId)}/manifest?${query.toString()}`
		);
	},
	packResources: (
		examId: string,
		body: PackResourcesRequest
	) =>
		call<PackResourcesResponse>(
			'POST',
			`/v2/packs/${encodeURIComponent(examId)}/resources`,
			body
		),
	revokePackLease: (leaseId: string) =>
		call<PackLeaseRevokedResponse>('DELETE', `/v1/packs/lease/${encodeURIComponent(leaseId)}`),
	listPackLeases: () => call<PackLeaseListResponse>('GET', '/v1/me/packs'),
	listScenarios: () => call<ScenarioListResponse>('GET', '/v1/scenarios'),
	imageCases: () => call<ImageCaseListResponse>('GET', '/v1/me/image-cases'),
	imageCase: (caseId: string) =>
		call<ImageCaseDetail>('GET', `/v1/me/image-cases/${encodeURIComponent(caseId)}`),
	adminImageCaseConcepts: (caseId: string) =>
		call<ImageCaseConceptsResponse>(
			'GET',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/concepts`
		),
	setAdminImageCaseConcepts: (caseId: string, conceptIds: string[]) =>
		call<ImageCaseConceptsResponse>(
			'PUT',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/concepts`,
			{ concept_ids: conceptIds } satisfies ImageCaseConceptsRequest
		),
	adminImageAnnotations: () =>
		call<AdminImageAnnotationListResponse>('GET', '/v1/admin/image-annotations'),
	createImageAnnotation: (
		caseId: string,
		body: ImageAnnotationRequest
	) =>
		call<ImageAnnotationCreatedResponse>(
			'POST',
			`/v1/admin/image-cases/${encodeURIComponent(caseId)}/annotations`,
			body
		),
	reviewImageAnnotation: (
		annotationId: string,
		decision: 'approved' | 'rejected',
		note: string
	) =>
		call<ImageAnnotationReviewResponse>(
			'POST',
			`/v1/admin/image-annotations/${encodeURIComponent(annotationId)}/review`,
			{ decision, note } satisfies ImageAnnotationReviewRequest
		),
	startScenario: (scenarioSlug: string) =>
		call<ScenarioStartResponse>('POST', '/v1/scenarios/runs', { scenario_slug: scenarioSlug }),
	getScenarioRun: (runId: string) =>
		call<ScenarioRun>('GET', `/v1/scenarios/runs/${encodeURIComponent(runId)}`),
	getScenarioTeam: (runId: string) =>
		call<ScenarioTeam>('GET', `/v1/scenarios/runs/${encodeURIComponent(runId)}/team`),
	createScenarioTeamInvite: (runId: string, role: Exclude<ScenarioTeamRole, 'team_lead'>) =>
		call<ScenarioTeamInviteCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/team/invites`,
			{ role }
		),
	joinScenarioTeam: (inviteCode: string) =>
		call<ScenarioTeamJoinResponse>(
			'POST',
			'/v1/scenario-team-invites/join',
			{ invite_code: inviteCode }
		),
	listScenarioHandovers: (runId: string) =>
		call<ScenarioHandoversResponse>(
			'GET',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers`
		),
	createScenarioHandover: (
		runId: string,
		body: Pick<ScenarioHandover, 'situation' | 'background' | 'assessment' | 'recommendation'> & {
			recipient_member_id: string;
		}
	) =>
		call<ScenarioHandoverCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers`,
			body
		),
	acknowledgeScenarioHandover: (runId: string, handoverId: string) =>
		call<ScenarioHandoverAcknowledgedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/handovers/${encodeURIComponent(handoverId)}/ack`
		),
	advanceScenario: (runId: string, event: string) =>
		call<ScenarioEventResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/events`,
			{ event }
		),
	getScenarioDebrief: (runId: string) =>
		call<ScenarioDebrief>(
			'GET',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/debrief`
		),
	appealScenarioAssessment: (runId: string, body: ScenarioAssessmentAppealRequest) =>
		call<ScenarioAssessmentAppealCreatedResponse>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/appeals`,
			body
		),
	replayScenario: (runId: string, events: string[]) =>
		call<ScenarioCounterfactualReplay>(
			'POST',
			`/v1/scenarios/runs/${encodeURIComponent(runId)}/counterfactual`,
			{ events }
		),
	listScenarioAssessmentAppeals: () =>
		call<ScenarioAssessmentAppealQueueResponse>(
			'GET',
			'/v1/admin/scenario-assessment-appeals'
		),
	getScenarioAssessmentAppeal: (appealId: string) =>
		call<ScenarioAssessmentAppeal>(
			'GET',
			`/v1/admin/scenario-assessment-appeals/${encodeURIComponent(appealId)}`
		),
	reviewScenarioAssessmentAppeal: (
		appealId: string,
		body: ScenarioAssessmentAppealReviewRequest
	) =>
		call<ScenarioAssessmentAppealReviewResponse>(
			'POST',
			`/v1/admin/scenario-assessment-appeals/${encodeURIComponent(appealId)}/review`,
			body
		),
	getSession: (sid: string) => call<PracticeSession>('GET', `/v1/practice/sessions/${sid}`),
	recordIntegrityEvent: (body: IntegrityEventRequest) =>
		call<IntegrityEventResponse>('POST', '/v1/integrity-events', body),
	getSessionHint: (sid: string, itemIndex: number) =>
		call<SessionHintResponse>(
			'GET',
			`/v1/practice/sessions/${sid}/items/${itemIndex}/hint`
		),
	calculate: (kind: string, inputs: CalculatorInput) =>
		call<CalculatorResponse>(
			'POST',
			`/v1/calculators/${encodeURIComponent(kind)}`,
			inputs
		),
	convertUnits: (body: ConvertInput) =>
		call<ConvertResponse>(
			'POST',
			'/v1/calculators/convert',
			body
		),
	answer: (sid: string, body: AnswerRequest) =>
		call<AnswerResponse>('POST', `/v1/practice/sessions/${sid}/answers`, body),
	submit: (sid: string) => call<SubmitReceipt>('POST', `/v1/practice/sessions/${sid}/submit`),
	undo: (planId: string, revisionId: string) =>
		call<UndoResponse>(
			'POST',
			`/v1/plans/${planId}/revisions/${revisionId}/undo`
		),
	reportQuestion: (versionId: string, body: ReportQuestionRequest) =>
		call<QuestionReportResponse>(
			'POST',
			`/v1/questions/versions/${versionId}/reports`,
			body
		),
	adminReports: () =>
		call<AdminReportsResponse>('GET', '/v1/admin/reports?limit=100'),
	adminDashboard: () => call<AdminDashboard>('GET', '/v1/admin/dashboard'),
	adminArticles: () => call<AdminArticleListResponse>('GET', '/v1/admin/articles'),
	createAdminArticle: (body: CreateArticleRequest) =>
		call<AdminArticleVersion>('POST', '/v1/admin/articles', body),
	createAdminArticleVersion: (articleId: string) =>
		call<AdminArticleVersion>(
			'POST',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions`
		),
	adminArticleVersion: (articleId: string, versionId: string) =>
		call<AdminArticleVersion>(
			'GET',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}`
		),
	updateAdminArticleDraft: (
		articleId: string,
		versionId: string,
		body: UpdateArticleDraftRequest
	) =>
		call<AdminArticleVersion>(
			'PATCH',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}`,
			body
		),
	publishAdminArticleDraft: (articleId: string, versionId: string) =>
		call<PublishArticleResponse>(
			'POST',
			`/v1/admin/articles/${encodeURIComponent(articleId)}/versions/${encodeURIComponent(versionId)}/publish`
		),
	adminSettings: () => call<AdminSettingsResponse>('GET', '/v1/admin/settings'),
	updateAdminSettings: (settings: AdminSettingsUpdateRequest) =>
		call<AdminSettingsUpdateResponse>('PATCH', '/v1/admin/settings', settings),
	listContentRights: () =>
		call<ContentRightsResponse>('GET', '/v1/admin/content-rights'),
	listExtractionReports: () =>
		call<AdminExtractionReportListResponse>('GET', '/v1/admin/library/extraction-reports'),
	pendingScenarioAssessments: () =>
		call<PendingScenarioAssessmentsResponse>(
			'GET',
			'/v1/admin/scenarios/runs/pending-assessment'
		),
	getScenarioAssessment: (runId: string) =>
		call<AdminScenarioAssessment>(
			'GET',
			`/v1/admin/scenarios/runs/${encodeURIComponent(runId)}/assessment`
		),
	recordScenarioAssessment: (
		runId: string,
		body: ScenarioAssessmentRequest
	) =>
		call<ScenarioAssessmentReceipt>(
			'POST',
			`/v1/admin/scenarios/runs/${encodeURIComponent(runId)}/assessment`,
			body
		),
	createExtractionReport: (body: CreateExtractionReportRequest) =>
		call<AdminExtractionReport>('POST', '/v1/admin/library/extraction-reports', body),
	reviewExtractionReport: (
		reportId: string,
		body: ReviewExtractionReportRequest
	) =>
		call<AdminExtractionReport>(
			'POST',
			`/v1/admin/library/extraction-reports/${encodeURIComponent(reportId)}/review`,
			body
		),
	createContentRights: (body: ContentRightsRequest) =>
		call<CreateContentRightsResponse>('POST', '/v1/admin/content-rights', body),
	revokeContentRights: (rightsId: string, reason: string) =>
		call<RevokeContentRightsResponse>(
			'PATCH',
			`/v1/admin/content-rights/${encodeURIComponent(rightsId)}/revoke`,
			{ reason } satisfies RevokeContentRightsRequest
		),
	resolveReport: (
		reportId: string,
		status: ResolveQuestionReportRequest['status'],
		resolutionNote: string,
		correctionNote?: string
	) =>
		call<ResolveQuestionReportResponse>(
			'POST',
			`/v1/reports/${encodeURIComponent(reportId)}/resolve`,
			{
				status,
				resolution_note: resolutionNote,
				...(correctionNote ? { correction_note: correctionNote } : {})
			} satisfies ResolveQuestionReportRequest
		),
	createDeck: (name: string) =>
		call<CreateDeckResponse>(
			'POST',
			'/v1/decks',
			{ name } satisfies CreateDeckRequest
		),
	addCard: (deckId: string, front: string, back: string) =>
		call<AddCardResponse>(
			'POST',
			`/v1/decks/${deckId}/cards`,
			{ front, back } satisfies AddCardRequest
		),
	reviewQueue: () =>
		call<ReviewQueueResponse>('GET', '/v1/reviews/queue'),
	listExams: () => call<ExamRegistryResponse>('GET', '/v1/exams'),
	listMocks: () => call<MockListResponse>('GET', '/v1/mocks'),
	createMock: (body: CreateMockRequest) =>
		call<CreateMockResponse>('POST', '/v1/mocks', body),
	listCompetitions: () =>
		call<CompetitionListResponse>('GET', '/v1/competitions'),
	competitionLeague: (examId: string) =>
		call<CompetitionLeagueState>(
			'GET',
			`/v1/leagues/${encodeURIComponent(examId)}`
		),
	joinCompetitionLeague: (examId: string) =>
		call<Extract<CompetitionLeagueState, { joined: true }>>(
			'POST',
			`/v1/leagues/${encodeURIComponent(examId)}/join`
		),
	leaveCompetitionLeague: (examId: string) =>
		call<LeaveCompetitionLeagueResponse>(
			'DELETE',
			`/v1/leagues/${encodeURIComponent(examId)}/membership`
		),
	startCompetitionEntry: (competitionId: string, handle: string) =>
		call<CompetitionAttemptStep>(
			'POST',
			`/v1/competitions/${encodeURIComponent(competitionId)}/entry`,
			{ handle } satisfies StartCompetitionEntryRequest
		),
	answerCompetitionQuestion: (
		competitionId: string,
		questionVersionId: string,
		chosenIndex: number,
		idempotencyKey: string
	) =>
		call<CompetitionAttemptStep>(
			'POST',
			`/v1/competitions/${encodeURIComponent(competitionId)}/entry/answer`,
			{
				question_version_id: questionVersionId,
				chosen_index: chosenIndex,
				idempotency_key: idempotencyKey
			} satisfies AnswerCompetitionQuestionRequest
		),
	competitionLeaderboard: (competitionId: string) =>
		call<CompetitionLeaderboardResponse>(
			'GET',
			`/v1/competitions/${encodeURIComponent(competitionId)}/leaderboard`
		),
	startMock: (mockId: string) =>
		call<StartMockResponse>('POST', `/v1/mocks/${mockId}/start`),
	answerableQuestions: () =>
		call<AnswerableQuestionsResponse>('GET', '/v1/coach/answerable-questions'),
	coachTurn: (
		vid: string,
		promptType: string,
		message: string,
		idempotencyKey: string
	) =>
		call<CoachTurnResponse>('POST', '/v1/coach/turns', {
			question_version_id: vid,
			prompt_type: promptType,
			message,
			idempotency_key: idempotencyKey
		} satisfies CoachTurnRequest),
	coachHistory: (vid: string) =>
		call<CoachHistoryResponse>(
			'GET',
			`/v1/coach/history?question_version_id=${vid}`
		),
	createNote: (title: string, body: string, sourceQuestionVersionId?: string) =>
		call<CreateNoteResponse>(
			'POST',
			'/v1/notes',
			{
				title,
				body,
				...(sourceQuestionVersionId
					? { source_question_version_id: sourceQuestionVersionId }
					: {})
			} satisfies NoteRequest
		),
	updateNote: (
		noteId: string,
		body: Pick<NoteRequest, 'title' | 'body' | 'base_updated_at'>
	) =>
		call<UpdateNoteResponse>('PATCH', `/v1/notes/${encodeURIComponent(noteId)}`, body),
	listNotes: () => call<ListNotesResponse>('GET', '/v1/notes'),
	myMarks: () =>
		call<MarkedQuestionsResponse>(
			'GET',
			'/v1/me/marks'
		),
	markQuestion: (versionId: string) =>
		call<MarkResponse>('POST', `/v1/questions/${encodeURIComponent(versionId)}/mark`),
	unmarkQuestion: (versionId: string) =>
		call<MarkResponse>('DELETE', `/v1/questions/${encodeURIComponent(versionId)}/mark`),
	deleteNote: (noteId: string) =>
		call<DeleteNoteResponse>('DELETE', `/v1/notes/${noteId}`),
	librarySearch: (q: string, scope: { jurisdiction?: string; as_of?: string } = {}) => {
		const params = new URLSearchParams({ q });
		if (scope.jurisdiction) params.set('jurisdiction', scope.jurisdiction);
		if (scope.as_of) params.set('as_of', scope.as_of);
		return call<LibrarySearchResponse>('GET', `/v1/library/search?${params.toString()}`);
	},
	libraryArticle: (slug: string, scope: { jurisdiction?: string; as_of?: string } = {}) => {
		const params = new URLSearchParams();
		if (scope.jurisdiction) params.set('jurisdiction', scope.jurisdiction);
		if (scope.as_of) params.set('as_of', scope.as_of);
		const suffix = params.toString();
		return call<LibraryArticleResponse>(
			'GET',
			`/v1/library/articles/${encodeURIComponent(slug)}${suffix ? `?${suffix}` : ''}`
		);
	},
	privateImportRights: () =>
		call<PrivateImportRightsResponse>('GET', '/v1/me/library/import-rights'),
	listPrivateImports: () =>
		call<PrivateImportListResponse>('GET', '/v1/me/library/imports'),
	createPrivateImport: (body: CreatePrivateImportRequest) =>
		call<PrivateImportSummary>('POST', '/v1/me/library/imports', body),
	getPrivateImport: (documentId: string) =>
		call<PrivateImportResponse>('GET', `/v1/me/library/imports/${encodeURIComponent(documentId)}`),
	deletePrivateImport: (documentId: string) =>
		call<PrivateImportDeletedResponse>('DELETE', `/v1/me/library/imports/${encodeURIComponent(documentId)}`),
	inbox: () => call<NotificationInboxResponse>('GET', '/v1/me/notifications'),
	updateNotificationPrefs: (body: NotificationPreferencesUpdateRequest) =>
		call<NotificationPreferencesUpdateResponse>('PATCH', '/v1/me/notifications', body),
	addPortfolioEntry: (body: PortfolioCreateRequest) =>
		call<PortfolioCreateResponse>('POST', '/v1/me/portfolio', body),
	listPortfolio: () => call<PortfolioListResponse>('GET', '/v1/me/portfolio'),
	addCeActivity: (activity: string, hours: number) =>
		call<CeActivityResponse>(
			'POST',
			'/v1/me/ce-activities',
			{ activity, hours } satisfies CeActivityRequest
		),
	listAdminAudit: () =>
		call<AdminAuditResponse>('GET', '/v1/admin/audit'),
	createNode: (body: CreateNodeRequest) =>
		call<CreateNodeResponse>('POST', '/v1/admin/hierarchy', body),
	adminHierarchy: (examId: string) =>
		call<AdminHierarchyResponse>(
			'GET',
			`/v1/admin/hierarchy?exam_id=${encodeURIComponent(examId)}`
		),
	adminConcepts: () => call<AdminConceptListResponse>('GET', '/v1/admin/concepts'),
	createConcept: (body: CreateConceptRequest) =>
		call<CreateConceptResponse>('POST', '/v1/admin/concepts', body),
	createConceptVersion: (
		conceptId: string,
		body: CreateConceptVersionRequest
	) =>
		call<CreateConceptVersionResponse>(
			'POST',
			`/v1/admin/concepts/${encodeURIComponent(conceptId)}/versions`,
			body
		),
	adminNodeConcepts: (nodeId: string) =>
		call<NodeConceptsResponse>(
			'GET',
			`/v1/admin/hierarchy/${encodeURIComponent(nodeId)}/concepts`
		),
	setAdminNodeConcepts: (nodeId: string, conceptIds: string[]) =>
		call<SetNodeConceptsResponse>(
			'PUT',
			`/v1/admin/hierarchy/${encodeURIComponent(nodeId)}/concepts`,
			{ concept_ids: conceptIds } satisfies SetNodeConceptsRequest
		),
	importQuestions: (body: AdminImportRequest) =>
		call<AdminImportResponse>('POST', '/v1/admin/import', body),
	importQuestionFile: (examId: string, dryRun: boolean, file: File, contentType: string) => {
		const query = new URLSearchParams({ exam_id: examId, dry_run: String(dryRun) });
		return call<AdminImportResponse>(
			'POST',
			`/v1/admin/import-file?${query.toString()}`,
			file,
			contentType
		);
	},
	rollbackImport: (batchId: string) =>
		call<RollbackImportResponse>(
			'POST',
			`/v1/admin/import/${batchId}/rollback`
		),
	assessmentWorkflow: (
		action: 'submit' | 'approve' | 'reject' | 'publish',
		versionIds: string[]
	) =>
		call<AssessmentWorkflowResponse>('POST', '/v1/admin/assessment-workflow', {
			action,
			version_ids: versionIds
		} satisfies AssessmentWorkflowRequest),
	institutionAnalytics: (institutionId: string, cohortId: string) => {
		const query = { cohort_id: cohortId } satisfies InstitutionAnalyticsQuery;
		const params = new URLSearchParams(query);
		return call<InstitutionAnalyticsResponse>(
			'GET',
			`/v1/institutions/${institutionId}/analytics?${params.toString()}`
		);
	},
	institutionAudit: (institutionId: string) =>
		call<InstitutionAuditResponse>(
			'GET',
			`/v1/institutions/${institutionId}/audit`
		),
	reviewEvent: (
		cardId: string,
		rating: ReviewEventRequest['rating'],
		idempotencyKey: string
	) =>
		call<ReviewEventResponse>(
			'POST',
			'/v1/reviews/events',
			{ card_id: cardId, rating, idempotency_key: idempotencyKey } satisfies ReviewEventRequest
		),
	reviewDebt: () => call<ReviewDebtResponse>('GET', '/v1/me/review-debt'),
	myCurriculum: () =>
		call<MyCurriculumResponse>('GET', '/v1/me/curriculum'),
	masteryHeatmap: (params?: HeatmapQuery) => {
		const qs = new URLSearchParams();
		if (params?.system_id) qs.set('system_id', params.system_id);
		if (params?.difficulty) qs.set('difficulty', params.difficulty);
		if (params?.trend_days) qs.set('trend_days', String(params.trend_days));
		const suffix = qs.toString() ? `?${qs.toString()}` : '';
		return call<MasteryHeatmapResponse>('GET', `/v1/me/heatmap${suffix}`);
	},
	myInstitutions: () =>
		call<MyInstitutionsResponse>('GET', '/v1/me/institutions'),
	createInstitution: (name: string) =>
		call<CreateInstitutionResponse>(
			'POST',
			'/v1/institutions',
			{ name } satisfies CreateInstitutionRequest
		),
	addInstitutionMember: (
		institutionId: string,
		userId: string,
		role: string
	) =>
		call<AddInstitutionMemberResponse>(
			'POST',
			`/v1/institutions/${institutionId}/members`,
			{ user_id: userId, role } satisfies AddInstitutionMemberRequest
		),
	institutionCohorts: (institutionId: string) =>
		call<InstitutionCohortsResponse>('GET', `/v1/institutions/${institutionId}/cohorts`),
	institutionPrograms: (institutionId: string) =>
		call<InstitutionProgramsResponse>('GET', `/v1/institutions/${institutionId}/programs`),
	createInstitutionProgram: (institutionId: string, name: string) =>
		call<CreateProgramResponse>(
			'POST',
			`/v1/institutions/${institutionId}/programs`,
			{ name } satisfies CreateProgramRequest
		),
	setProgramCurriculum: (institutionId: string, programId: string, chapterIds: string[]) =>
		call<SetProgramCurriculumResponse>(
			'PUT',
			`/v1/institutions/${institutionId}/programs/${programId}/curriculum`,
			{ chapter_ids: chapterIds } satisfies SetProgramCurriculumRequest
		),
	programCurriculumCoverage: (institutionId: string, programId: string) =>
		call<ProgramCurriculumCoverageResponse>(
			'GET',
			`/v1/institutions/${institutionId}/programs/${programId}/coverage`
		),
	createCohort: (
		institutionId: string,
		name: string,
		memberIds: string[],
		programId?: string
	) =>
		call<CreateCohortResponse>(
			'POST',
			`/v1/institutions/${institutionId}/cohorts`,
			{ name, member_ids: memberIds, ...(programId ? { program_id: programId } : {}) } satisfies CreateCohortRequest
		),
	createAssignment: (cohortId: string, title: string, dueAt?: string) =>
		call<CreateAssignmentResponse>(
			'POST',
			`/v1/cohorts/${cohortId}/assignments`,
			{ title, due_at: dueAt } satisfies CreateAssignmentRequest
		),
	communityProfile: () =>
		call<CommunityProfileResponse>('GET', '/v1/community/me'),
	createCommunityProfile: (handle: string) =>
		call<CreateCommunityProfileResponse>(
			'POST',
			'/v1/community/profile',
			{ handle } satisfies CommunityProfileRequest
		),
	profileByHandle: (handle: string) =>
		call<CommunityProfileByHandleResponse>(
			'GET',
			`/v1/community/profiles/${handle}`
		),
	createDuel: (opponent: string, examId: string, questionCount: number, chapterId?: string) =>
		call<CreateDuelResponse>(
			'POST',
			'/v1/community/duels',
			{
				opponent,
				exam_id: examId,
				question_count: questionCount,
				...(chapterId ? { chapter_id: chapterId } : {})
			} satisfies CreateDuelRequest
		),
	duelByToken: (token: string) =>
		call<DuelByTokenResponse>(
			'GET',
			`/v1/community/duels/by-token/${encodeURIComponent(token)}`
		),
	myDuels: () => call<MyDuelsResponse>('GET', '/v1/me/duels'),
	shareCards: () => call<ShareCardsResponse>('GET', '/v1/me/share-cards'),
	duelState: (duelId: string) =>
		call<DuelStateResponse>(
			'GET',
			`/v1/community/duels/${duelId}`
		),
	acceptDuel: (duelId: string) =>
		call<AcceptDuelResponse>(
			'POST',
			`/v1/community/duels/${duelId}/accept`
		),
	declineDuel: (duelId: string) =>
		call<DeclineDuelResponse>(
			'POST',
			`/v1/community/duels/${duelId}/decline`
		),
	createCommunityGroup: (name: string) =>
		call<CreateCommunityGroupResponse>(
			'POST',
			'/v1/community/groups',
			{ name } satisfies CreateCommunityGroupRequest
		),
	joinGroup: (groupId: string) =>
		call<JoinCommunityGroupResponse>('POST', `/v1/community/groups/${groupId}/join`),
	listGroupPosts: (groupId: string) =>
		call<CommunityPostListResponse>(
			'GET',
			`/v1/community/groups/${groupId}/posts`
		),
	createGroupPost: (groupId: string, body: string) =>
		call<CreateCommunityPostResponse>(
			'POST',
			`/v1/community/groups/${groupId}/posts`,
			{ body } satisfies CreateCommunityPostRequest
		),
	removeGroupPost: (groupId: string, postId: string) =>
		call<RemoveCommunityPostResponse>(
			'DELETE',
			`/v1/community/groups/${groupId}/posts/${postId}`
		),
	reportGroupPost: (
		groupId: string,
		postId: string,
		reason: ReportCommunityPostRequest['reason'],
		note: string
	) =>
		call<ReportCommunityPostResponse>(
			'POST',
			`/v1/community/groups/${groupId}/posts/${postId}/reports`,
			{ reason, note: note.trim() || null } satisfies ReportCommunityPostRequest
		),
	myCommunityPostReports: () =>
		call<MyCommunityPostReportsResponse>('GET', '/v1/community/me/reports'),
	groupPostReportQueue: (groupId: string) =>
		call<GroupPostReportQueueResponse>('GET', `/v1/community/groups/${groupId}/reports`),
	resolveGroupPostReport: (
		groupId: string,
		reportId: string,
		action: ResolveCommunityPostReportRequest['action']
	) =>
		call<ResolveCommunityPostReportResponse>(
			'POST',
			`/v1/community/groups/${groupId}/reports/${reportId}/resolve`,
			{ action } satisfies ResolveCommunityPostReportRequest
		),
	listGroups: () =>
		call<CommunityGroupsResponse>('GET', '/v1/community/groups'),
	selectionPolicy: () =>
		call<SelectionPolicyResponse>('GET', '/v1/me/selection-policy')
};
